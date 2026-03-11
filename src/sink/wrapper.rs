use std::sync::LazyLock;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::sink::{LogUpdate, Sink};

static ASYNC_SINK_HANDLERS: LazyLock<Mutex<Vec<thread::JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(Vec::new()));

enum AsyncSinkOp {
	Flush,
	Write { update: LogUpdate },
}

pub struct AsyncSink {
	name: String,
	receives_all_levels: bool,
	tx: mpsc::Sender<AsyncSinkOp>,
	rx_handler_idx: usize,
}

impl<'s> AsyncSink {
	pub fn new<T: Sink + 's>(sink: T) -> Self {
		let name = sink.name().into();
		let receives_all_levels = sink.receives_all_levels();

		let (tx, rx) = mpsc::channel::<AsyncSinkOp>();

		let rx_handler = thread::spawn(move || {
			/*
			while let Ok(cmd) = rx.recv() {
				match cmd {
					AsyncSinkOp::Flush => sink.flush(),
					AsyncSinkOp::Write { update: update } => sink.write(&update),
				}
			}
			*/
		});

		let rx_handler_idx: usize;
		match ASYNC_SINK_HANDLERS.lock() {
			Ok(mut v) => {
				v.push(rx_handler);
				rx_handler_idx = v.len() - 1;
			}
			Err(e) => panic!("failed to acquire lock on async sink handlers list: {e}"),
		}

		Self {
			name: name,
			receives_all_levels,
			tx: tx,
			rx_handler_idx: rx_handler_idx,
		}
	}
}

impl Sink for AsyncSink {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn write(&mut self, update: &LogUpdate) {
		match self.tx.send(AsyncSinkOp::Write { update: update.clone() }) {
			Ok(_) => (),
			Err(e) => panic!("failed to send update to log sink \"{name}\": {e}", name = self.name),
		};
	}

	fn flush(&mut self) {
		match self.tx.send(AsyncSinkOp::Flush) {
			Ok(_) => (),
			Err(e) => panic!("failed to send flush to log sink \"{name}\": {e}", name = self.name),
		};
	}

	fn drop(&self) {
		//self.tx.close();
		/*
		match ASYNC_SINK_HANDLERS.lock() {
			Ok(mut v) => {
				match v.get(self.rx_handler_idx) {
					Some(handler) => match (&handler).join() {
						Ok => (),
						Err(e) => panic!(
							"failed to join async sink handler thread #{}",
							self.rx_handler_idx
						),
					},
					None => panic!(
						"mising async sink handlers #{idx}",
						idx = self.rx_handler_idx
					),
				};
			}
			Err(e) => panic!("failed to acquire lock on async sink handlers list: {e}"),
		}
		*/
	}

	fn receives_all_levels(&self) -> bool {
		self.receives_all_levels
	}
}
