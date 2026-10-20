use ntime::{Timestamp, sleep};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use crate::attributes;
use crate::constant::{DEFAULT_CAPACITY, THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::SinkRef;

static ASYNC_QUEUE_CAPACITY: usize = 4 * DEFAULT_CAPACITY;
static GLOBAL_ASYNC_HANDLER: LazyLock<AsyncHandler> = LazyLock::new(|| AsyncHandler::new());

// Encapsulates all supported asynchronous operations, so they can be reused without heap reallocs.
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

	pub fn is_set(&mut self) -> bool {
		self.sink.is_some()
	}

	pub fn unset(&mut self) {
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

// Lockless, thread-safe async operations ring buffer queue, implemented as an array of reusable data structs.
pub struct AsyncQueue {
	// mutex-ed async op slots are overkill, as the way AsyncQueue works guarantees no slot
	// will ever be concurrently read nor written. Mutex'es however cleanly enable mutable slots
	// within non-mutable methods - without the need of UnsafeCell and the likes.
	pool: [Mutex<AsyncSinkOp>; ASYNC_QUEUE_CAPACITY],
	// use only head and tail indeces for the queue; this enables fully concurrent read/writes
	// without additional atomics, but caps the max queue size at (capacity - 1).
	head_idx: AtomicUsize,
	tail_idx: AtomicUsize,
}

impl AsyncQueue {
	pub fn new() -> Self {
		let mut pool_vec: Vec<Mutex<AsyncSinkOp>> = Vec::with_capacity(ASYNC_QUEUE_CAPACITY);
		for _ in 0..ASYNC_QUEUE_CAPACITY {
			pool_vec.push(Mutex::new(AsyncSinkOp::new()));
		}
		let pool: [Mutex<AsyncSinkOp>; ASYNC_QUEUE_CAPACITY] = match pool_vec.try_into() {
			Ok(a) => a,
			Err(_) => panic!("failed to initialize AsyncQueue pool array"),
		};

		AsyncQueue {
			pool: pool,
			head_idx: AtomicUsize::new(0),
			tail_idx: AtomicUsize::new(0),
		}
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.pool.len()
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		head_idx == tail_idx
	}

	#[inline]
	pub fn is_full(&self) -> bool {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		tail_idx == (head_idx + 1) % self.capacity()
	}

	#[inline]
	pub fn len(&self) -> usize {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		// TODO: fix me
		match tail_idx >= head_idx {
			true => tail_idx - head_idx,
			false => self.capacity() - head_idx - tail_idx,
		}
	}

	fn push_back<'f>(&'f self) -> Option<&'f Mutex<AsyncSinkOp>> {
		let mut pre_tail_idx: usize;

		loop {
			pre_tail_idx = self.tail_idx.load(Ordering::SeqCst);
			let tail_idx = (pre_tail_idx + 1) % self.capacity();
			if tail_idx == self.head_idx.load(Ordering::SeqCst) {
				// queue is full
				return None;
			}

			match self.tail_idx.compare_exchange(pre_tail_idx, tail_idx, Ordering::SeqCst, Ordering::Relaxed) {
				Ok(_) => break,
				Err(_) => continue,
			};
		}

		Some(&self.pool[pre_tail_idx])
	}

	pub fn push_back_flush<'f>(&'f self, sink: &SinkRef) -> bool {
		match &mut self.push_back() {
			None => false,
			Some(op) => {
				op.lock().unwrap().set_flush(sink);
				true
			}
		}
	}

	pub fn push_back_log_update<'f>(&'f self, sink: &SinkRef, update: &LogUpdate) -> bool {
		match self.push_back() {
			None => false,
			Some(op) => {
				op.lock().unwrap().set_log_update(sink, update);
				true
			}
		}
	}

	pub fn get_front<'f>(&'f self) -> Option<&'f Mutex<AsyncSinkOp>> {
		match self.is_empty() {
			true => None,
			false => {
				let head_idx = self.head_idx.load(Ordering::Relaxed);
				Some(&self.pool[head_idx])
			}
		}
	}

	pub fn pop_front(&self) {
		loop {
			let pre_head_idx = self.head_idx.load(Ordering::SeqCst);
			if pre_head_idx == self.tail_idx.load(Ordering::SeqCst) {
				// queue is empty
				return;
			}
			let head_idx = (pre_head_idx + 1) % self.capacity();

			match self.head_idx.compare_exchange(pre_head_idx, head_idx, Ordering::SeqCst, Ordering::Relaxed) {
				Ok(_) => break,
				Err(_) => continue,
			};
		}
	}
}

struct AsyncHandler {
	queue: Arc<AsyncQueue>,
	actuator: Mutex<Option<thread::JoinHandle<()>>>,
	closed: Arc<AtomicBool>,
	refcount: AtomicUsize,
}

impl AsyncHandler {
	fn new() -> Self {
		Self {
			queue: Arc::new(AsyncQueue::new()),
			actuator: Mutex::new(None),
			closed: Arc::new(AtomicBool::new(false)),
			refcount: AtomicUsize::new(0),
		}
	}

	fn closed(&self) -> bool {
		self.closed.load(Ordering::Relaxed)
	}

	fn start_actuator(&self) {
		let mut actuator_lock = self.actuator.lock().unwrap();
		if actuator_lock.is_some() {
			return;
		}

		self.closed.store(false, Ordering::SeqCst);

		let queue_ref = self.queue.clone();
		let closed_ref = self.closed.clone();
		let actuator = thread::spawn(move || {
			loop {
				// wait for log ops to be queued...
				if queue_ref.is_empty() {
					if closed_ref.load(Ordering::Relaxed) {
						break;
					}
					thread::yield_now();
					continue;
				}

				// ...then process them.
				loop {
					let op_locked = match queue_ref.get_front() {
						None => break,
						Some(ol) => ol,
					};
					let mut op = op_locked.lock().unwrap();

					// account for the possible race conditions of an op read happening after it's
					// assigned a pool slot, bue before that slot is actually written to.
					if !op.is_set() {
						continue;
					}

					if let Some(sink) = op.get_flush() {
						match sink.lock() {
							Ok(mut s) => match s.flush() {
								Ok(_) => (),
								Err(e) => panic!("async flush on sink {name} failed: {e}", name = s.name()),
							},
							Err(e) => panic!("failed to acquire lock on sink: {e}"),
						};
					} else if let Some((sink, update)) = op.get_log_update() {
						match sink.lock() {
							Ok(mut s) => match s.log(&update) {
								Ok(_) => (),
								Err(e) => panic!("async log update {update:?} on sink {name} failed: {e}", name = s.name()),
							},
							Err(e) => panic!("failed to acquire lock on sink: {e}"),
						};
					} else {
						panic!("invalid async operation");
					}

					op.unset();
					queue_ref.pop_front();
				}
			}
		});

		*actuator_lock = Some(actuator);
	}

	fn stop_actuator(&self) {
		let mut actuator_lock = self.actuator.lock().unwrap();
		let actuator = match &*actuator_lock {
			Some(a) => a,
			None => return,
		};

		// close this handler...
		self.closed.store(true, Ordering::SeqCst);

		// ...and busy wait for the async queue actuator to complete, with a timeout. this
		// prevents potential sink write issues from locking shutdown requesters forever.
		let start = Timestamp::now();
		loop {
			if actuator.is_finished() {
				break;
			}

			if Timestamp::now().diff_as_duration(&start) > THREAD_FINALIZE_TIMEOUT {
				panic!(
					"failed to shut down AsyncHandler queue after {wait:?} with {queue_len}/{queue_capacity} ops left",
					wait = THREAD_FINALIZE_TIMEOUT,
					queue_len = self.queue.len(),
					queue_capacity = self.queue.capacity(),
				);
			}
			sleep(THREAD_FINALIZE_SPINLOCK_WAIT);
			thread::yield_now();
		}

		*actuator_lock = None;
	}

	pub fn refcount(&self) -> usize {
		self.refcount.load(Ordering::Relaxed)
	}

	pub fn inc_refcount(&self) {
		loop {
			let rc = self.refcount.load(Ordering::Acquire);

			let new_rc = rc + 1;
			if self.refcount.compare_exchange(rc, new_rc, Ordering::Release, Ordering::Relaxed).is_ok() {
				if rc == 0 {
					self.start_actuator();
				}
				break;
			}
		}
	}

	pub fn dec_refcount(&self) {
		loop {
			let rc = self.refcount.load(Ordering::Acquire);
			if rc == 0 {
				return;
			}

			let new_rc = rc - 1;
			if self.refcount.compare_exchange(rc, new_rc, Ordering::Release, Ordering::Relaxed).is_ok() {
				if rc == 1 {
					self.stop_actuator();
				}
				break;
			}
		}
	}

	#[inline]
	fn panic_if_closed(&self) {
		if !self.closed() {
			return;
		}

		panic!(
			"attempted to push into closed AsyncHandler with {len}/{capacity} elements",
			len = self.queue.len(),
			capacity = self.queue.capacity()
		);
	}

	fn log(&self, sink: &SinkRef, update: &LogUpdate) {
		self.panic_if_closed();
		while !self.queue.push_back_log_update(sink, update) {}
	}

	fn flush(&self, sink: &SinkRef) {
		self.panic_if_closed();
		while !self.queue.push_back_flush(sink) {}
	}
}

impl Default for AsyncHandler {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for AsyncHandler {
	fn drop(&mut self) {
		self.stop_actuator()
	}
}

/// Returns the number of active loggers referencing the global async handler.
pub fn refcount() -> usize {
	(*GLOBAL_ASYNC_HANDLER).refcount()
}

/// Increments the count of active loggers referencing the global async handler.
pub fn inc_refcount() {
	(*GLOBAL_ASYNC_HANDLER).inc_refcount()
}

/// Decrements the count of active loggers referencing the global async handler.
pub fn dec_refcount() {
	(*GLOBAL_ASYNC_HANDLER).dec_refcount()
}

/// Queues a log operation for the async handler.
pub fn log(sink: &SinkRef, update: &LogUpdate) {
	(*GLOBAL_ASYNC_HANDLER).log(sink, update)
}

/// Queues a sink flush operation for the async handler.
pub fn flush(sink: &SinkRef) {
	(*GLOBAL_ASYNC_HANDLER).flush(sink)
}
