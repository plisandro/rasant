use ntime::{Timestamp, sleep};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;

use crate::attributes;
use crate::constant::{THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::{AsyncSinkSender, SinkRef};

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncSinkHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u32> = Mutex::new(0);

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
		Self::new()
	}
}

impl<'i> AsyncSinkOp {
	#[inline]
	pub fn new() -> Self {
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

struct AsyncSinkHandler {
	tx: Option<AsyncSinkSender>,
	rx_handler: Option<thread::JoinHandle<()>>,
}

impl AsyncSinkHandler {
	fn new() -> Self {
		let (tx, rx) = mpsc::channel::<AsyncSinkOp>();

		let rx_handler = thread::spawn(move || {
			while let Ok(op) = rx.recv() {
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
		});

		Self {
			tx: Some(tx),
			rx_handler: Some(rx_handler),
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

/// Returns an operation sender channel for the async handler.
pub fn get_sender() -> AsyncSinkSender {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().get_sender()
}

/// Queues a log operation for the async handler.
pub fn log(tx: &AsyncSinkSender, sink: &SinkRef, update: &LogUpdate) {
	let mut op = AsyncSinkOp::new();
	op.set_log_update(sink, update);

	match tx.send(op) {
		Ok(_) => (),
		Err(e) => {
			let sink_name = sink.lock().unwrap().name().to_string();
			panic!("failed to queue log update {update:?} on {sink_name}: {e}");
		}
	};
}

/// Queues a sink flush operation for the async handler.
pub fn flush(tx: &AsyncSinkSender, sink: &SinkRef) {
	let mut op = AsyncSinkOp::new();
	op.set_flush(sink);

	match tx.send(op) {
		Ok(_) => (),
		Err(e) => {
			let sink_name = sink.lock().unwrap().name().to_string();
			panic!("failed to queue flush on {sink_name}: {e}");
		}
	};
}
