use ntime::{Timestamp, sleep};
use std::cell::UnsafeCell;
use std::hint;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use crate::attributes;
use crate::constant::{CAPACITY_ASYNC_QUEUE, THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::{CachePadded, SinkRef};

static GLOBAL_ASYNC_HANDLER: LazyLock<AsyncHandler> = LazyLock::new(|| AsyncHandler::new());

// Encapsulates all supported asynchronous operations, so they can be reused without heap reallocs.
pub struct AsyncSinkOp {
	// Target sink for this operation.
	sink: Option<SinkRef>,
	// Whether the operation is fully initialized and ready for a get().
	ready: AtomicBool,

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
			ready: AtomicBool::new(false),
			flush: false,
			when: Timestamp::epoch(),
			level: Level::Info,
			depth: 0,
			msg: String::from(""),
			attrs: attributes::Map::new(),
		}
	}

	pub fn is_set(&mut self) -> bool {
		self.ready.load(Ordering::Acquire)
	}

	pub fn unset(&mut self) {
		self.sink = None;
		self.ready.store(false, Ordering::Release);
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
		self.flush = true;
		self.sink = Some(sink.clone());

		self.ready.store(true, Ordering::Release);
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

		self.ready.store(true, Ordering::Release);
	}
}

// Lockless, thread-safe async operations ring buffer MPSC queue, implemented as an array of reusable data structs.
pub struct AsyncQueue {
	pool: [UnsafeCell<AsyncSinkOp>; CAPACITY_ASYNC_QUEUE],
	head_idx: CachePadded<AtomicUsize>,
	tail_idx: CachePadded<AtomicUsize>,
}

// SAFETY: AsyncQueue will never try to push/pop to the same UnsafeCell-ed pool slots
// concurrently, so it is safe to share between threads.
unsafe impl Sync for AsyncQueue {}

impl AsyncQueue {
	pub fn new() -> Self {
		let mut pool_vec: Vec<UnsafeCell<AsyncSinkOp>> = Vec::with_capacity(CAPACITY_ASYNC_QUEUE);
		for _ in 0..CAPACITY_ASYNC_QUEUE {
			pool_vec.push(UnsafeCell::new(AsyncSinkOp::new()));
		}
		let pool = match pool_vec.try_into() {
			Ok(a) => a,
			Err(_) => panic!("failed to initialize AsyncQueue pool array"),
		};

		AsyncQueue {
			pool: pool,
			head_idx: CachePadded::new(AtomicUsize::new(0)),
			tail_idx: CachePadded::new(AtomicUsize::new(0)),
		}
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		// we use only head and tail indeces for the queue; this enables fully
		// concurrent read/writes without additional atomics, but caps the max
		// queue size at (pool_capacity - 1).
		self.pool.len() - 1
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		head_idx == tail_idx
	}

	#[inline]
	pub fn len(&self) -> usize {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		match tail_idx >= head_idx {
			true => tail_idx - head_idx,
			false => tail_idx + self.pool.len() - head_idx,
		}
	}

	fn push_back<'f>(&'f self) -> Option<&'f mut AsyncSinkOp> {
		let mut pre_tail_idx: usize;

		loop {
			pre_tail_idx = self.tail_idx.load(Ordering::Acquire);
			let tail_idx = (pre_tail_idx + 1) % self.pool.len();
			if tail_idx == self.head_idx.load(Ordering::SeqCst) {
				// queue is full
				return None;
			}

			match self.tail_idx.compare_exchange(pre_tail_idx, tail_idx, Ordering::Release, Ordering::Relaxed) {
				Ok(_) => break,
				Err(_) => continue,
			};
		}

		// SAFETY: read and write operations are guaranteed to never hapen concurrently
		// on the same pool slot, so ops can be safely accessed concurrently as mutable.
		Some(unsafe { &mut *self.pool[pre_tail_idx].get() })
	}

	pub fn push_back_flush<'f>(&'f self, sink: &SinkRef) {
		loop {
			hint::spin_loop();
			if let Some(op) = self.push_back() {
				op.set_flush(sink);
				break;
			}
		}
	}

	pub fn push_back_log_update<'f>(&'f self, sink: &SinkRef, update: &LogUpdate) {
		loop {
			hint::spin_loop();
			if let Some(op) = self.push_back() {
				op.set_log_update(sink, update);
				break;
			}
		}
	}

	pub fn get_front<'f>(&'f self) -> Option<&'f mut AsyncSinkOp> {
		let head_idx = self.head_idx.load(Ordering::Relaxed);
		let tail_idx = self.tail_idx.load(Ordering::Relaxed);

		if head_idx == tail_idx {
			// empty queue
			return None;
		}

		// SAFETY: read and write operations are guaranteed to never hapen concurrently
		// on the same pool slot, so ops can be safely accessed concurrently as mutable.
		let elem = unsafe { &mut *self.pool[head_idx].get() };

		// account for possible race conditions between an AsyncSinkOp slot being
		// requested for write, and actually being fully updated.
		if !elem.is_set() {
			return None;
		}

		Some(elem)
	}

	pub fn pop_front(&self) -> bool {
		loop {
			let pre_head_idx = self.head_idx.load(Ordering::Acquire);
			if pre_head_idx == self.tail_idx.load(Ordering::SeqCst) {
				// queue is empty
				return false;
			}
			let head_idx = (pre_head_idx + 1) % self.pool.len();

			match self.head_idx.compare_exchange(pre_head_idx, head_idx, Ordering::Release, Ordering::Relaxed) {
				Ok(_) => break,
				Err(_) => continue,
			};
		}

		true
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
					hint::spin_loop();
					continue;
				}

				// ...then process them.
				while let Some(op) = queue_ref.get_front() {
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
		self.queue.push_back_log_update(sink, update);
	}

	fn flush(&self, sink: &SinkRef) {
		self.panic_if_closed();
		self.queue.push_back_flush(sink);
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

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod queue_tests {
	use crate::sink::black_hole;

	use super::*;

	#[test]
	fn high_contention_single_consumer() {
		static WRITER_THREADS: usize = 100;
		static WRITER_ITERATIONS: usize = 500;

		let queue = Arc::new(AsyncQueue::new());
		let sink: SinkRef = Arc::new(Mutex::new(Box::new(black_hole::default())));
		let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

		// multiple writer threads...
		for _ in 0..WRITER_THREADS {
			let queue_ref = queue.clone();
			let sink_ref = sink.clone();
			handles.push(thread::spawn(move || {
				for _ in 0..WRITER_ITERATIONS {
					queue_ref.push_back_flush(&sink_ref);
				}
			}));
		}

		// ..and a single consumer thread.
		let queue_ref = queue.clone();
		handles.push(thread::spawn(move || {
			for _ in 0..(WRITER_THREADS * WRITER_ITERATIONS) {
				while !queue_ref.pop_front() {
					thread::yield_now();
				}
			}
		}));

		for h in handles {
			assert_eq!(h.join().is_ok(), true);
		}

		assert!(queue.is_empty());
		assert_eq!(queue.len(), 0);
	}
	#[test]

	fn high_contention_multiple_consumers() {
		static THREADS: usize = 100;
		static WRITER_ITERATIONS: usize = 10000;

		let queue = Arc::new(AsyncQueue::new());
		let sink: SinkRef = Arc::new(Mutex::new(Box::new(black_hole::default())));
		let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

		// multiple writer threads...
		for _ in 0..(THREADS / 2) {
			let queue_ref = queue.clone();
			let sink_ref = sink.clone();
			handles.push(thread::spawn(move || {
				for _ in 0..WRITER_ITERATIONS {
					queue_ref.push_back_flush(&sink_ref);
				}
			}));
		}

		// ..and multiple consumer threads, reading in parallel.
		for _ in 0..(THREADS / 2) {
			let queue_ref = queue.clone();
			handles.push(thread::spawn(move || {
				for _ in 0..WRITER_ITERATIONS {
					while !queue_ref.pop_front() {
						thread::yield_now();
					}
				}
			}));
		}

		for h in handles {
			assert_eq!(h.join().is_ok(), true);
		}

		assert!(queue.is_empty());
		assert_eq!(queue.len(), 0);
	}
}
