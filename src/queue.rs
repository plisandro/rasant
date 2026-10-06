use ntime::{Timestamp, sleep};
use std::mem;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::attributes;
use crate::constant::{THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::{AsyncSinkSender, SinkRef};

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncSinkHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u32> = Mutex::new(0);

/// Supported asynchronous operations.
pub enum AsyncSinkOp {
	Log { sink: SinkRef },
	FlushSink { sink: SinkRef },
}

pub struct AsyncLogQueueEntry {
	pub when: Timestamp,
	pub level: Level,
	pub depth: LogDepth,
	pub msg: String,
	pub attrs: attributes::Map,
}

impl Default for AsyncLogQueueEntry {
	fn default() -> Self {
		Self {
			when: Timestamp::epoch(),
			level: Level::Info,
			depth: 0,
			msg: String::from(""),
			attrs: attributes::Map::new(),
		}
	}
}

impl<'i> From<&'i LogUpdate<'i>> for AsyncLogQueueEntry {
	fn from(update: &'i LogUpdate) -> Self {
		AsyncLogQueueEntry {
			when: update.when().clone(),
			level: update.level().clone(),
			depth: update.depth().clone(),
			msg: String::from(update.message()),
			attrs: update.attributes(),
		}
	}
}

impl<'i> AsyncLogQueueEntry {
	pub fn copy_from_log_update(&mut self, update: &'i LogUpdate<'i>) {
		self.when.copy_from(update.when());
		self.level = update.level().clone();
		self.depth = update.depth().clone();
		self.msg.clear();
		self.msg.push_str(update.message());
		update.copy_attributes_into(&mut self.attrs);
	}
}

// Async log update queue, implemented as a FIFO ring buffer with reusable update data structs.
pub struct AsyncLogQueue {
	pub pool: Vec<AsyncLogQueueEntry>,
	pub head_idx: usize,
	pub tail_idx: usize,
	pub size: usize,
}

impl AsyncLogQueue {
	pub fn new() -> Self {
		AsyncLogQueue {
			// TODO: preallocate some capacity for the queue.
			pool: Vec::new(),
			head_idx: 0,
			tail_idx: 0,
			size: 0,
		}
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.size
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.pool.len()
	}

	pub fn push_back<'f>(&'f mut self, update: &LogUpdate) {
		let mut capacity = self.capacity();

		if self.size >= capacity {
			// no more pool space, extend it.
			self.pool.insert(self.tail_idx, AsyncLogQueueEntry::from(update));
			capacity += 1;

			self.size += 1;
			self.head_idx = (self.head_idx + 1) % capacity;
			self.tail_idx = self.head_idx;

			return;
		}

		// reuse an existing log queue slot
		match self.pool.get_mut(self.tail_idx) {
			None => panic!("failed to push update {update:?} into async log queue"),
			Some(e) => e.copy_from_log_update(update),
		};
		self.size += 1;
		self.tail_idx = (self.tail_idx + 1) % capacity;
	}

	pub fn pop_front<'f>(&'f mut self) -> Option<&'f mut AsyncLogQueueEntry> {
		if self.size == 0 {
			return None;
		}

		let capacity = self.capacity();
		let entry = self.pool.get_mut(self.head_idx);

		self.size -= 1;
		self.head_idx = (self.head_idx + 1) % capacity;

		entry
	}
}

struct AsyncSinkHandler {
	tx: Option<AsyncSinkSender>,
	rx_handler: Option<thread::JoinHandle<()>>,
	log_update_pool: Arc<Mutex<AsyncLogQueue>>,
}

impl AsyncSinkHandler {
	fn new() -> Self {
		let (tx, rx) = mpsc::channel::<AsyncSinkOp>();
		let pool = Arc::new(Mutex::new(AsyncLogQueue::new()));
		let pool_ref = pool.clone();

		let rx_handler = thread::spawn(move || {
			let mut common_entry = AsyncLogQueueEntry::default();
			while let Ok(cmd) = rx.recv() {
				match cmd {
					AsyncSinkOp::Log { sink } => {
						match pool_ref.lock() {
							Ok(mut p) => {
								let pool_entry = match p.pop_front() {
									Some(m) => m,
									None => panic!("no entry for queued async log update on sink {name}", name = sink.lock().unwrap().name()),
								};
								mem::swap(&mut common_entry, pool_entry);
							}
							Err(e) => panic!("failed to acquire async log queue lock: {e}"),
						};

						let update = LogUpdate::from((&common_entry.when, common_entry.level, common_entry.depth, common_entry.msg.as_str(), &common_entry.attrs));
						match sink.lock() {
							Ok(mut s) => match s.log(&update) {
								Ok(_) => (),
								Err(e) => panic!("async log update {update:?} on sink {name} failed: {e}", name = s.name()),
							},
							Err(e) => panic!("failed to acquire lock on sink: {e}"),
						}
					}
					AsyncSinkOp::FlushSink { sink } => match sink.lock() {
						Ok(mut s) => match s.flush() {
							Ok(_) => (),
							Err(e) => panic!("async flush on sink {name} failed: {e}", name = s.name()),
						},
						Err(e) => panic!("failed to acquire lock on sink: {e}"),
					},
				};
			}
		});

		Self {
			tx: Some(tx),
			rx_handler: Some(rx_handler),
			log_update_pool: pool,
		}
	}

	fn get_sender(&self) -> AsyncSinkSender {
		match self.tx {
			Some(ref tx) => tx.clone(),
			None => panic!("tried to get a sender for a closed async queue handler"),
		}
	}

	fn shutdown(&mut self) {
		// close the main async queue sender and wait for the handler thread to die
		self.tx = None;

		// we don't join() the handler thread, to prevent any potential issues causing a deadlock during shutdown.
		// if we fail to kill the handler after a period of time, panic the process instead.
		match self.rx_handler.take() {
			None => panic!("tried to shut down a closed sync queue handler"),
			Some(rx_handler) => {
				let start = Timestamp::now();
				while !rx_handler.is_finished() {
					if Timestamp::now().diff_as_duration(&start) > THREAD_FINALIZE_TIMEOUT {
						panic!(
							"failed to shut down AsyncSinkHandler after {wait:?} with {refcount} async loggers",
							wait = THREAD_FINALIZE_TIMEOUT,
							refcount = refcount()
						);
					};
					sleep(THREAD_FINALIZE_SPINLOCK_WAIT);
					thread::yield_now();
				}
			}
		};
	}

	fn log(&mut self, sink: &SinkRef, update: &LogUpdate) {
		let mut pool = self.log_update_pool.lock().unwrap();
		pool.push_back(update);

		match self.get_sender().send(AsyncSinkOp::Log { sink: sink.clone() }) {
			Ok(_) => (),
			Err(e) => {
				let sink_name = sink.lock().unwrap().name().to_string();
				panic!("failed to queue log update {update:?} on {sink_name}: {e}");
			}
		}
	}

	fn flush(&mut self, sink: &SinkRef) {
		match self.get_sender().send(AsyncSinkOp::FlushSink { sink: sink.clone() }) {
			Ok(_) => (),
			Err(e) => {
				let sink_name = sink.lock().unwrap().name().to_string();
				panic!("failed to queue flush on {sink_name}: {e}");
			}
		};
	}
}

impl Default for AsyncSinkHandler {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for AsyncSinkHandler {
	fn drop(&mut self) {
		self.shutdown()
	}
}

fn drop() {
	*(GLOBAL_ASYNC_HANDLER.lock().unwrap()) = None;
}

/// Returns the number of active loggers referencing the global async handler.
pub fn refcount() -> u32 {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap())
}

/// Increments the count of active loggers referencing the global async handler.
pub fn inc_refcount() {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap()) += 1;
}

/// Decrements the count of active loggers referencing the global async handler.
pub fn dec_refcount() {
	let no_loggers: bool;
	{
		let mut count = GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap();
		if *count == 0 {
			panic!("async loggers count decremented below zero");
		}
		*count -= 1;
		no_loggers = *count == 0;
	}

	if no_loggers {
		// force handler shutdown once no loggers are referencing the async queue
		drop();
	}
}

/// Queues a log operation for the async handler.
pub fn log(sink: &SinkRef, update: &LogUpdate) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().log(sink, update)
}

/// Queues a sink flush operation for the async handler.
pub fn flush(sink: &SinkRef) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().flush(sink)
}
