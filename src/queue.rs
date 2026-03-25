use std::hint;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::attributes;
use crate::sink::{LogUpdate, Sink};
use crate::time::{Duration, Timestamp, sleep};

const ASYNC_HANDLER_OP_TIMEOUT: Duration = Duration::from_secs(10);
const ASYNC_HANDLER_SPINLOCK_WAIT: Duration = Duration::from_millis(50);

static GLOBAL_ASYNC_HANDLER: Mutex<Option<AsyncSinkHandler>> = Mutex::new(None);
static GLOBAL_ASYNC_HANDLER_REFCOUNT: Mutex<u32> = Mutex::new(0);

enum AsyncSinkOp {
	Log {
		sink: Arc<Mutex<Box<dyn Sink + Send>>>,
		update: LogUpdate,
		attrs: attributes::Map,
	},
	FlushSink {
		sink: Arc<Mutex<Box<dyn Sink + Send>>>,
	},
}

struct AsyncSinkHandler {
	tx: mpsc::Sender<AsyncSinkOp>,
	rx_handler: Option<thread::JoinHandle<()>>,
	size: Arc<Mutex<usize>>,
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
			tx: tx,
			rx_handler: Some(rx_handler),
			size: size,
		}
	}

	fn queue_size(&self) -> usize {
		*(self.size.lock().unwrap())
	}

	fn log(&self, sink: Arc<Mutex<Box<dyn Sink + Send>>>, update: LogUpdate, attrs: attributes::Map) {
		*(self.size.lock().unwrap()) += 1;
		match self.tx.send(AsyncSinkOp::Log {
			sink: sink,
			update: update,
			attrs: attrs,
		}) {
			Ok(_) => (),
			Err(e) => panic!("failed to queue log update: {e}"),
		};
	}

	fn flush_sink(&self, sink: Arc<Mutex<Box<dyn Sink + Send>>>) {
		*(self.size.lock().unwrap()) += 1;
		match self.tx.send(AsyncSinkOp::FlushSink { sink: sink }) {
			Ok(_) => (),
			Err(e) => panic!("failed to queue sink flush: {e}"),
		};
	}

	fn flush_queue(&self) {
		let start = Timestamp::now();
		while self.queue_size() != 0 {
			if Timestamp::now().diff_as_duration(&start) > ASYNC_HANDLER_OP_TIMEOUT {
				panic!(
					"failed to flush AsyncSinkHanlder after {wait:?}, {size} ops left",
					wait = ASYNC_HANDLER_OP_TIMEOUT,
					size = self.queue_size()
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

		if let Some(rx_handler) = self.rx_handler.take() {
			match rx_handler.join() {
				Ok(_) => (),
				Err(e) => panic!("failed to close async log handler: {e:?}"),
			};
		};
	}
}

fn drop() {
	//*(GLOBAL_ASYNC_HANDLER.lock().unwrap()) = None;
}

pub fn refcount() -> u32 {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap())
}

pub fn inc_refcount() {
	*(GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap()) += 1;
}

pub fn dec_refcount() {
	let mut count = GLOBAL_ASYNC_HANDLER_REFCOUNT.lock().unwrap();
	if *count == 0 {
		panic!("async loggers count decremented below zero");
	}
	*count -= 1;

	if *count == 0 {
		// shutdown handler if no clients are left
		drop();
	}
}

pub fn size() -> usize {
	match GLOBAL_ASYNC_HANDLER.lock().unwrap().as_ref() {
		Some(h) => h.queue_size(),
		None => 0,
	}
}

pub fn flush() {
	match GLOBAL_ASYNC_HANDLER.lock().unwrap().as_ref() {
		Some(h) => h.flush_queue(),
		None => (),
	};
}

pub fn log(sink: &Arc<Mutex<Box<dyn Sink + Send>>>, update: &LogUpdate, attrs: &attributes::Map) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().log(sink.clone(), update.clone(), attrs.clone())
}

pub fn flush_sink(sink: &Arc<Mutex<Box<dyn Sink + Send>>>) {
	GLOBAL_ASYNC_HANDLER.lock().unwrap().get_or_insert_default().flush_sink(sink.clone())
}
