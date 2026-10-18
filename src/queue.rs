use ntime::{Timestamp, sleep};
use std::mem;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::attributes;
use crate::constant::{DEFAULT_CAPACITY, THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::SinkRef;

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u64> = Mutex::new(0);

// Encapsulates all supported asynchronous operations so they can be reused without heap reallocs.
pub struct AsyncSinkOp {
	sink: Option<SinkRef>,

	// Whether the operation is a log sink flush.
	flush: bool,

	// Whether the operation is a log write.
	when: Timestamp,
	level: Level,
	depth: LogDepth,
	msg: String,
	attrs: attributes::Map,
}

impl Default for AsyncSinkOp {
	fn default() -> Self {
		Self {
			sink: None,
			flush: false,
			when: Timestamp::epoch(),
			level: Level::Info,
			depth: 0,
			msg: String::from(""),
			attrs: attributes::Map::new(),
		}
	}
}

impl<'i> AsyncSinkOp {
	pub fn reset(&mut self) {
		self.sink = None;
	}

	pub fn get_flush(&self) -> Option<SinkRef> {
		match self.sink {
			None => None,
			Some(ref sink) => match self.flush {
				false => None,
				true => Some(sink.clone()),
			},
		}
	}

	pub fn get_log_update<'f>(&'f self) -> Option<(SinkRef, LogUpdate<'f>)> {
		match self.sink {
			None => None,
			Some(ref sink) => match self.flush {
				true => None,
				false => Some((sink.clone(), LogUpdate::from((&self.when, self.level, self.depth, self.msg.as_str(), &self.attrs)))),
			},
		}
	}

	pub fn set_flush(&mut self, sink: &SinkRef) {
		self.sink = Some(sink.clone());
		self.flush = true;
	}

	pub fn set_log_update(&mut self, sink: &SinkRef, update: &'i LogUpdate<'i>) {
		self.sink = Some(sink.clone());
		self.flush = false;
		self.when.copy_from(update.when());
		self.level = update.level().clone();
		self.depth = update.depth().clone();
		self.msg.clear();
		self.msg.push_str(update.message());
		update.copy_attributes_into(&mut self.attrs);
	}
}

// Async operations queue, implemented as a FIFO ring buffer with reusable data structs.
pub struct AsyncQueue {
	pool: Vec<AsyncSinkOp>,
	closed: bool,
	head_idx: usize,
	tail_idx: usize,
	size: usize,
}

impl AsyncQueue {
	pub fn new() -> Self {
		let mut pool: Vec<AsyncSinkOp> = Vec::with_capacity(DEFAULT_CAPACITY);
		for _ in 0..DEFAULT_CAPACITY {
			pool.push(AsyncSinkOp::default());
		}

		AsyncQueue {
			pool: pool,
			closed: false,
			head_idx: 0,
			tail_idx: 0,
			size: 0,
		}
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		return self.size == 0;
	}

	#[inline]
	pub fn is_closed(&self) -> bool {
		return self.closed;
	}

	#[inline]
	pub fn is_done(&self) -> bool {
		return self.closed && self.size == 0;
	}

	#[inline]
	pub fn is_ready(&self) -> bool {
		return self.closed || self.size != 0;
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.size
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.pool.len()
	}

	pub fn close(&mut self) {
		self.closed = true;
	}

	fn push_back<'f>(&'f mut self) -> &'f mut AsyncSinkOp {
		if self.closed {
			panic!("tried to push on a closed AsyncQueue with {len}/{capacity} elements", len = self.len(), capacity = self.capacity());
		}

		let mut capacity = self.capacity();

		if self.size >= capacity {
			// no more pool space, extend it.
			let entry = self.pool.insert_mut(self.tail_idx, AsyncSinkOp::default());
			capacity += 1;

			self.size += 1;
			self.head_idx = (self.head_idx + 1) % capacity;
			self.tail_idx = self.head_idx;

			return entry;
		}

		// reuse an existing queue slot
		let entry = match self.pool.get_mut(self.tail_idx) {
			None => panic!("failed to push into async log queue"),
			Some(e) => e,
		};
		self.size += 1;
		self.tail_idx = (self.tail_idx + 1) % capacity;

		entry
	}

	pub fn push_back_flush<'f>(&'f mut self, sink: &SinkRef) {
		let entry: &mut AsyncSinkOp = self.push_back();
		entry.set_flush(sink);
	}

	pub fn push_back_log_update<'f>(&'f mut self, sink: &SinkRef, update: &LogUpdate) {
		let entry: &mut AsyncSinkOp = self.push_back();
		entry.set_log_update(sink, update);
	}

	pub fn pop_front<'f>(&'f mut self) -> Option<&'f mut AsyncSinkOp> {
		if self.size == 0 {
			return None;
		}

		let capacity = self.capacity();
		match self.pool.get_mut(self.head_idx) {
			None => None,
			Some(entry) => {
				self.size -= 1;
				self.head_idx = (self.head_idx + 1) % capacity;

				Some(entry)
			}
		}
	}
}

struct AsyncHandler {
	queue: Arc<Mutex<AsyncQueue>>,
	queue_cvar: Arc<Condvar>,
	actuator: thread::JoinHandle<()>,
}

impl AsyncHandler {
	fn new() -> Self {
		let queue_cvar = Arc::new(Condvar::new());
		let queue_cvar_ref = queue_cvar.clone();
		let queue = Arc::new(Mutex::new(AsyncQueue::new()));
		let queue_ref = queue.clone();

		let actuator = thread::spawn(move || {
			let mut op_queue = AsyncQueue::new();
			loop {
				{
					let mut queue = queue_cvar_ref.wait_while(queue_ref.lock().unwrap(), |q| !q.is_ready()).unwrap();
					if queue.is_done() {
						break;
					}
					if queue.is_empty() {
						continue;
					}

					mem::swap(&mut *queue, &mut op_queue);
					if op_queue.is_closed() {
						// ensure the closed state is retained after buffer swaps
						queue.close();
					}
				}

				while let Some(op) = op_queue.pop_front() {
					if let Some(sink) = op.get_flush() {
						match sink.lock() {
							Ok(mut s) => match s.flush() {
								Ok(_) => (),
								Err(e) => panic!("async flush on sink {name} failed: {e}", name = s.name()),
							},
							Err(e) => panic!("failed to acquire lock on sink: {e}"),
						};
						continue;
					}

					if let Some((sink, update)) = op.get_log_update() {
						match sink.lock() {
							Ok(mut s) => match s.log(&update) {
								Ok(_) => (),
								Err(e) => panic!("async log update {update:?} on sink {name} failed: {e}", name = s.name()),
							},
							Err(e) => panic!("failed to acquire lock on sink: {e}"),
						};
						continue;
					}

					panic!("invalid async operation");
				}
			}
		});

		Self {
			queue: queue,
			queue_cvar: queue_cvar,
			actuator: actuator,
		}
	}

	fn log(&mut self, sink: &SinkRef, update: &LogUpdate) {
		self.queue.lock().unwrap().push_back_log_update(sink, update);
		self.queue_cvar.notify_one();
	}

	fn flush(&mut self, sink: &SinkRef) {
		self.queue.lock().unwrap().push_back_flush(sink);
		self.queue_cvar.notify_one();
	}

	fn shutdown(&mut self) {
		// close the main async queue sender and wait for the handler thread to die
		self.queue.lock().unwrap().close();
		self.queue_cvar.notify_all();

		// we busy wait for the async queue to flush completely, with a timeout. this prevents
		// potential sink write issues from locking shutdown requester forever.
		let start = Timestamp::now();
		loop {
			if self.queue.lock().unwrap().is_done() {
				break;
			}

			if Timestamp::now().diff_as_duration(&start) > THREAD_FINALIZE_TIMEOUT {
				let queue_lock = self.queue.lock().unwrap();
				panic!(
					"failed to shut down AsyncHandler queue after {wait:?} with {queue_len}/{queue_capacity} ops left",
					wait = THREAD_FINALIZE_TIMEOUT,
					queue_len = queue_lock.len(),
					queue_capacity = queue_lock.capacity(),
				);
			}
			sleep(THREAD_FINALIZE_SPINLOCK_WAIT);
			thread::yield_now();
		}
	}
}

impl Default for AsyncHandler {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for AsyncHandler {
	fn drop(&mut self) {
		self.shutdown()
	}
}

fn drop() {
	*(GLOBAL_ASYNC_HANDLER.lock().unwrap()) = None;
}

/// Returns the number of active loggers referencing the global async handler.
pub fn refcount() -> u64 {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap())
}

/// Increments the count of active loggers referencing the global async handler.
pub fn inc_refcount() {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap()) += 1;
}

/// Decrements the count of active loggers referencing the global async handler.
pub fn dec_refcount() {
	let mut count = GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap();
	if *count == 0 {
		panic!("async loggers count decremented below zero");
	}
	*count -= 1;

	if *count == 0 {
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
