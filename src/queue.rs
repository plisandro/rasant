use ntime::{Timestamp, sleep};
use std::hint;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::thread;

use crate::attributes;
use crate::constant::{DEFAULT_CAPACITY, THREAD_FINALIZE_SPINLOCK_WAIT, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::SinkRef;

static GLOBAL_ASYNC_HANDLER: LazyLock<AsyncHandler> = LazyLock::new(|| AsyncHandler::new());

// Encapsulates all supported asynchronous operations, so they can be reused without heap reallocs.
pub struct AsyncSinkOp {
	// Target sink for this operation.
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
		self.flush = true;
		self.sink = Some(sink.clone());
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

// Thread-safe async operations MPSC queue, implemented as an vector of reusable data structs.
pub struct AsyncQueue {
	// (ops vector, used length)
	pool: Mutex<(Vec<AsyncSinkOp>, usize)>,
}

/*
// A AsyncSinkOp iterator for queues.
pub struct AsyncQueueIter<'s> {
	pool_lock: MutexGuard<'s, (Vec<AsyncSinkOp>, usize)>,
	idx: usize,
}

impl<'i> AsyncQueueIter<'i> {
	pub fn new(queue: &'i AsyncQueue) -> Self {
		Self {
			pool_lock: queue.pool.lock().unwrap(),
			idx: 0,
		}
	}
}

impl<'i> Iterator for AsyncQueueIter<'i> {
	type Item = &'i mut AsyncSinkOp;

	fn next(&mut self) -> Option<Self::Item> {
		if self.idx >= self.pool_lock.1 {
			return None;
		}

		let idx = self.idx;
		self.idx += 1;

		Some(&mut self.pool_lock.0[idx])
	}
}
*/

impl AsyncQueue {
	pub fn new() -> Self {
		let mut pool_vec: Vec<AsyncSinkOp> = Vec::with_capacity(DEFAULT_CAPACITY);
		for _ in 0..DEFAULT_CAPACITY {
			pool_vec.push(AsyncSinkOp::new());
		}

		AsyncQueue { pool: Mutex::new((pool_vec, 0)) }
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.pool.lock().unwrap().0.len()
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.pool.lock().unwrap().1 == 0
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.pool.lock().unwrap().1
	}

	#[inline]
	pub fn clear(&self) {
		self.pool.lock().unwrap().1 = 0;
	}

	fn push<'f, C>(&'f self, mut op_callback: C)
	where
		C: FnMut(&mut AsyncSinkOp),
	{
		let mut pool_lock = self.pool.lock().unwrap();

		if pool_lock.1 == pool_lock.0.len() {
			// queue is full, resize
			pool_lock.0.reserve(DEFAULT_CAPACITY);
			for _ in 0..DEFAULT_CAPACITY {
				pool_lock.0.push(AsyncSinkOp::new());
			}
		}

		let idx = pool_lock.1;
		op_callback(&mut pool_lock.0[idx]);
		pool_lock.1 += 1;
	}

	pub fn push_flush<'f>(&'f self, sink: &SinkRef) {
		self.push(|op| op.set_flush(sink))
	}

	pub fn push_log_update<'f>(&'f self, sink: &SinkRef, update: &LogUpdate) {
		self.push(|op| op.set_log_update(sink, update))
	}

	pub fn swap(&self, other: &Self) {
		let mut self_pool_lock = self.pool.lock().unwrap();
		let mut other_pool_lock = other.pool.lock().unwrap();

		mem::swap(&mut self_pool_lock.0, &mut other_pool_lock.0);
		mem::swap(&mut self_pool_lock.1, &mut other_pool_lock.1);
	}

	pub fn lock<'f>(&'f self) -> MutexGuard<'f, (Vec<AsyncSinkOp>, usize)> {
		self.pool.lock().unwrap()
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
			let queue = AsyncQueue::new();

			loop {
				// TODO: fixme!
				// wait for log ops to be queued...
				if queue_ref.is_empty() {
					if closed_ref.load(Ordering::Relaxed) {
						break;
					}
					hint::spin_loop();
					continue;
				}

				// ...then process them.
				queue.swap(&queue_ref);

				let mut queue_lock = queue.lock();
				let size = queue_lock.1;
				for op in &mut queue_lock.0.as_mut_slice()[0..size] {
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

					// drop any lingering references in the async op - notably, Sink's.
					op.unset();
				}

				//queue.clear();
				queue_lock.1 = 0;
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

			if Timestamp::now().diff_as_duration(&start) > Ok(THREAD_FINALIZE_TIMEOUT) {
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
		self.queue.push_log_update(sink, update);
	}

	fn flush(&self, sink: &SinkRef) {
		self.panic_if_closed();
		self.queue.push_flush(sink);
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

/*
#[cfg(test)]
mod queue_tests {
	use crate::sink::black_hole;

	use super::*;

	#[test]
	fn high_contention_single_consumer() {
		static WRITER_THREADS: usize = 50;
		static WRITER_ITERATIONS: usize = 1000;

		let queue = Arc::new(AsyncQueue::new());
		let sink: SinkRef = Arc::new(Mutex::new(Box::new(black_hole::default())));
		let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

		// multiple writer threads...
		for _ in 0..WRITER_THREADS {
			let queue_ref = queue.clone();
			let sink_ref = sink.clone();
			handles.push(thread::spawn(move || {
				for _ in 0..WRITER_ITERATIONS {
					queue_ref.push_flush(&sink_ref);
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
		static TOTAL_THREADS: usize = 100;
		static WRITER_ITERATIONS: usize = 1000;

		let queue = Arc::new(AsyncQueue::new());
		let sink: SinkRef = Arc::new(Mutex::new(Box::new(black_hole::default())));
		let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();

		// multiple writer threads...
		for _ in 0..(TOTAL_THREADS / 2) {
			let queue_ref = queue.clone();
			let sink_ref = sink.clone();
			handles.push(thread::spawn(move || {
				for _ in 0..WRITER_ITERATIONS {
					queue_ref.push_back_flush(&sink_ref);
				}
			}));
		}

		// ..and multiple consumer threads, reading in parallel.
		for _ in 0..(TOTAL_THREADS / 2) {
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
*/
