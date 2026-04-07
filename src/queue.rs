use ntime::{Duration, Timestamp, sleep};
use std::hint;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::attributes;
use crate::sink::LogUpdate;
use crate::types::SinkRef;

const ASYNC_HANDLER_OP_TIMEOUT: Duration = Duration::from_secs(10);
const ASYNC_HANDLER_SPINLOCK_WAIT: Duration = Duration::from_millis(50);

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncSinkHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u32> = Mutex::new(0);

enum AsyncSinkOp {
	Log { sink: SinkRef, update: LogUpdate, attrs: attributes::Map },
	FlushSink { sink: SinkRef },
}

struct AsyncSinkHandler {
	tx: Option<mpsc::Sender<AsyncSinkOp>>,
	rx_handler: Option<thread::JoinHandle<()>>,
	queue_size: Arc<Mutex<usize>>,
}

impl AsyncSinkHandler {
	fn new() -> Self {
		let (tx, rx) = mpsc::channel::<AsyncSinkOp>();
		let size = Arc::new(Mutex::new(0 as usize));

		let asize = size.clone();
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

				match asize.lock() {
					Ok(mut s) => {
						if *s == 0 {
							panic!("processed AsyncSinkOp from a suposedly exhausted channel");
						}
						*s -= 1;
					}
					Err(e) => panic!("failed to acquire AsyncSinkOp count lock: {e}"),
				};
			}
		});

		Self {
			tx: Some(tx),
			rx_handler: Some(rx_handler),
			queue_size: size,
		}
	}

	fn get_queue_size(&self) -> usize {
		*(self.queue_size.lock().unwrap())
	}

	fn inc_queue_size(&self) {
		*(self.queue_size.lock().unwrap()) += 1;
	}

	fn log(&self, sink: SinkRef, update: LogUpdate, attrs: attributes::Map) {
		let Some(ref tx) = self.tx else {
			return;
		};
		self.inc_queue_size();

		match tx.send(AsyncSinkOp::Log {
			sink: sink,
			update: update,
			attrs: attrs,
		}) {
			Ok(_) => (),
			Err(e) => panic!("failed to queue log update: {e}"),
		};
	}

	fn flush_sink(&self, sink: SinkRef) {
		let Some(ref tx) = self.tx else {
			return;
		};
		self.inc_queue_size();

		match tx.send(AsyncSinkOp::FlushSink { sink: sink }) {
			Ok(_) => (),
			Err(e) => panic!("failed to queue sink flush: {e}"),
		};
	}

	fn flush_queue(&self) {
		let start = Timestamp::now();
		while self.get_queue_size() != 0 {
			if Timestamp::now().diff_as_duration(&start) > ASYNC_HANDLER_OP_TIMEOUT {
				panic!(
					"failed to flush AsyncSinkHanlder after {wait:?}, {size} ops left",
					wait = ASYNC_HANDLER_OP_TIMEOUT,
					size = self.get_queue_size()
				);
			}
			sleep(ASYNC_HANDLER_SPINLOCK_WAIT);
			hint::spin_loop();
		}
	}
}

impl Default for AsyncSinkHandler {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for AsyncSinkHandler {
	fn drop(&mut self) {
		self.flush_queue();

		// close the async queue sender and wait for the handler thread to die
		self.tx = None;
		if let Some(rx_handler) = self.rx_handler.take() {
			match rx_handler.join() {
				Ok(_) => (),
				Err(e) => panic!("failed to close async log handler: {e:?}"),
			};
		};
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
		// shutdown handler once no loggers are referencing the async queue
		drop();
	}
}

/// Returns the async queue size.
pub fn size() -> usize {
	match GLOBAL_ASYNC_HANDLER.lock().unwrap().as_ref() {
		Some(h) => h.get_queue_size(),
		None => 0,
	}
}

/// Flushes all pending async queue operations, locking until completion.
pub fn flush() {
	match GLOBAL_ASYNC_HANDLER.lock().unwrap().as_ref() {
		Some(h) => h.flush_queue(),
		None => (),
	};
}

/// Queues a log operation on the async queue.
pub fn log(sink: &SinkRef, update: &LogUpdate, attrs: &attributes::Map) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().log(sink.clone(), update.clone(), attrs.clone())
}

/// Queues a sink flush operation on the async queue.
pub fn flush_sink(sink: &SinkRef) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().flush_sink(sink.clone())
}
