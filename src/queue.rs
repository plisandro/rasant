use ntime::{Timestamp, sleep};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;

use crate::attributes;
use crate::constant::{THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::sink::LogUpdate;
use crate::types::{AsyncSinkSender, SinkRef};

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncSinkHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u32> = Mutex::new(0);

pub enum AsyncSinkOp {
	// TODO: allow for multiple sinks in the same Log op
	Log { sink: SinkRef, update: LogUpdate, attrs: attributes::Map },
	FlushSink { sink: SinkRef },
}

struct AsyncSinkHandler {
	tx: Option<AsyncSinkSender>,
	rx_handler: Option<thread::JoinHandle<()>>,
}

impl AsyncSinkHandler {
	fn new() -> Self {
		let (tx, rx) = mpsc::channel::<AsyncSinkOp>();

		let rx_handler = thread::spawn(move || {
			while let Ok(cmd) = rx.recv() {
				match cmd {
					AsyncSinkOp::Log { sink, update, attrs } => match sink.lock() {
						Ok(mut s) => match s.log(&update, &attrs) {
							Ok(_) => (),
							Err(e) => panic!("async log update {update:?} on sink {name} failed: {e}", name = s.name()),
						},
						Err(e) => panic!("failed to acquire lock on sink: {e}"),
					},
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
						panic!("failed to shut downh AsyncSinkHanlder after {wait:?}", wait = THREAD_FINALIZE_TIMEOUT);
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

/// Returns an operation sender channel for the async handler.
pub fn get_sender() -> AsyncSinkSender {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().get_sender()
}

/// Queues a log operation for the async handler.
pub fn log(tx: &AsyncSinkSender, sink: &SinkRef, update: &LogUpdate, attrs: &attributes::Map) {
	match tx.send(AsyncSinkOp::Log {
		sink: sink.clone(),
		update: update.clone(),
		attrs: attrs.clone(),
	}) {
		Ok(_) => (),
		Err(e) => {
			let sink_name = sink.lock().unwrap().name().to_string();
			panic!("failed to queue log update {update:?} + {attrs} on {sink_name}: {e}");
		}
	};
}

/// Queues a sink flush operation for the async handler.
pub fn flush(tx: &AsyncSinkSender, sink: &SinkRef) {
	match tx.send(AsyncSinkOp::FlushSink { sink: sink.clone() }) {
		Ok(_) => (),
		Err(e) => {
			let sink_name = sink.lock().unwrap().name().to_string();
			panic!("failed to queue flush on {sink_name}: {e}");
		}
	};
}
