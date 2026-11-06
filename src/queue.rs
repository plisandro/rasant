use ntime::{Timestamp, sleep};
use std::hint;
use std::mem;
use std::sync::RwLockReadGuard;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};
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

// Thread-safe, low-contention async operations MPSC queue, implemented as an vector of reusable data structs.
pub struct AsyncQueue {
	// TODO: use UnsafeCell instead of Mutexes for interior mutability?
	pool: RwLock<Vec<Mutex<AsyncSinkOp>>>,
	size: AtomicUsize,
	write_lock: AtomicBool,
	pending_pushes: AtomicUsize,
}

// A AsyncSinkOp iterator for AsyncQueues. Note this is *NOT* an Iterator implementation, as that
// trait would require self-borrows to work.
pub struct AsyncQueueIter<'s> {
	pool: RwLockReadGuard<'s, Vec<Mutex<AsyncSinkOp>>>,
	size: usize,
	idx: usize,
}

impl<'i> AsyncQueueIter<'i> {
	pub fn new(queue: &'i AsyncQueue) -> Self {
		Self {
			pool: queue.pool.read().unwrap(),
			size: queue.len(),
			idx: 0,
		}
	}

	pub fn next<'f>(&'f mut self) -> Option<&'f Mutex<AsyncSinkOp>> {
		if self.idx >= self.size {
			return None;
		}

		let slot = &self.pool[self.idx];
		self.idx += 1;
		Some(slot)
	}
}

impl AsyncQueue {
	pub fn new() -> Self {
		let mut pool: Vec<Mutex<AsyncSinkOp>> = Vec::with_capacity(DEFAULT_CAPACITY);
		for _ in 0..DEFAULT_CAPACITY {
			pool.push(Mutex::new(AsyncSinkOp::new()));
		}

		AsyncQueue {
			pool: RwLock::new(pool),
			size: AtomicUsize::new(0),
			write_lock: AtomicBool::new(false),
			pending_pushes: AtomicUsize::new(0),
		}
	}

	#[inline]
	pub fn iter<'f>(&'f self) -> AsyncQueueIter<'f> {
		AsyncQueueIter::new(self)
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.size.load(Ordering::Relaxed) == 0
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.size.load(Ordering::Relaxed)
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.pool.read().unwrap().len()
	}

	#[inline]
	pub fn clear(&self) {
		self.size.store(0, Ordering::SeqCst);
		self.pending_pushes.store(0, Ordering::SeqCst);
		self.unlock_writes();
	}

	#[inline]
	pub fn lock_writes(&self) {
		self.write_lock.store(true, Ordering::SeqCst);

		// wait for pending pushes to complete
		while self.pending_pushes.load(Ordering::SeqCst) != 0 {
			hint::spin_loop();
		}
	}

	#[inline]
	pub fn unlock_writes(&self) {
		self.write_lock.store(false, Ordering::SeqCst);
	}

	#[inline]
	pub fn writes_locked(&self) -> bool {
		self.write_lock.load(Ordering::SeqCst)
	}

	fn push<'f, C>(&'f self, mut op_callback: C)
	where
		C: FnMut(&mut AsyncSinkOp),
	{
		loop {
			if self.writes_locked() {
				hint::spin_loop();
				continue;
			}
			self.pending_pushes.fetch_add(1, Ordering::SeqCst);

			// double-check the write lock to prevent race conditions
			// between swap()s and concurrent push()es.
			if !self.writes_locked() {
				break;
			}
			self.pending_pushes.fetch_sub(1, Ordering::SeqCst);
			hint::spin_loop();
		}

		let idx = self.size.fetch_add(1, Ordering::AcqRel);
		if idx >= self.capacity() {
			// buffer is full, extend it (accounting for concurrent writes)
			let mut pool = self.pool.write().unwrap();
			while idx >= pool.len() {
				pool.reserve(DEFAULT_CAPACITY);
				for _ in 0..DEFAULT_CAPACITY {
					pool.push(Mutex::new(AsyncSinkOp::new()));
				}
			}
		}

		let slot = &self.pool.read().unwrap()[idx];
		op_callback(&mut slot.lock().unwrap());

		self.pending_pushes.fetch_sub(1, Ordering::Release);
	}

	pub fn push_flush<'f>(&'f self, sink: &SinkRef) {
		self.push(|op| op.set_flush(sink))
	}

	pub fn push_log_update<'f>(&'f self, sink: &SinkRef, update: &LogUpdate) {
		self.push(|op| op.set_log_update(sink, update))
	}

	pub fn swap(&self, other: &Self) {
		self.lock_writes();
		other.lock_writes();

		let mut self_pool = self.pool.write().unwrap();
		let self_size = self.size.load(Ordering::Acquire);
		let mut other_pool = other.pool.write().unwrap();
		let other_size = other.size.load(Ordering::Acquire);

		mem::swap(&mut *self_pool, &mut *other_pool);
		self.size.store(other_size, Ordering::Release);
		other.size.store(self_size, Ordering::Release);

		self.unlock_writes();
		other.unlock_writes();
	}
}

struct AsyncHandler {
	queue_main: Arc<AsyncQueue>,
	queue_write: Arc<AsyncQueue>,
	actuator: Mutex<Option<thread::JoinHandle<()>>>,
	closed: Arc<AtomicBool>,
	refcount: AtomicUsize,
}

impl AsyncHandler {
	fn new() -> Self {
		Self {
			queue_main: Arc::new(AsyncQueue::new()),
			queue_write: Arc::new(AsyncQueue::new()),
			actuator: Mutex::new(None),
			closed: Arc::new(AtomicBool::new(false)),
			refcount: AtomicUsize::new(0),
		}
	}

	fn closed(&self) -> bool {
		self.closed.load(Ordering::Relaxed)
	}

	pub fn queue_len(&self) -> usize {
		self.queue_main.len() + self.queue_write.len()
	}

	pub fn queue_capacity(&self) -> usize {
		self.queue_main.capacity() + self.queue_write.capacity()
	}

	fn start_actuator(&self) {
		let mut actuator_lock = self.actuator.lock().unwrap();
		if actuator_lock.is_some() {
			return;
		}

		self.closed.store(false, Ordering::SeqCst);

		let queue_main_ref = self.queue_main.clone();
		let queue_write_ref = self.queue_write.clone();
		let closed_ref = self.closed.clone();
		let actuator = thread::spawn(move || {
			loop {
				// TODO: rework to have the thread sleep instead of busy-waiting.
				// wait for log ops to be queued...
				if queue_main_ref.is_empty() {
					if closed_ref.load(Ordering::Relaxed) {
						break;
					}
					hint::spin_loop();
					continue;
				}

				// ...then process them.
				queue_write_ref.swap(&queue_main_ref);

				let mut ops = AsyncQueueIter::new(&queue_write_ref);
				while let Some(mop) = ops.next() {
					let mut op = mop.lock().unwrap();
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

				queue_write_ref.clear();
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
					"failed to shut down AsyncHandler queue after {wait:?} with {len}/{capacity} ops left",
					wait = THREAD_FINALIZE_TIMEOUT,
					len = self.queue_len(),
					capacity = self.queue_capacity(),
				);
			}
			sleep(THREAD_FINALIZE_SPINLOCK_WAIT);
			thread::yield_now();
		}

		*actuator_lock = None;
	}

	pub fn is_active(&self) -> bool {
		self.actuator.lock().unwrap().is_some()
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
		if self.closed() {
			panic!(
				"attempted to push into closed AsyncHandler with {len}/{capacity} elements",
				len = self.queue_len(),
				capacity = self.queue_capacity(),
			);
		}
	}

	fn log(&self, sink: &SinkRef, update: &LogUpdate) {
		self.panic_if_closed();
		self.queue_main.push_log_update(sink, update);
	}

	fn flush(&self, sink: &SinkRef) {
		self.panic_if_closed();
		self.queue_main.push_flush(sink);
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

/// Returns the number of log operations queued for processing by the global async handler.
pub fn len() -> usize {
	(*GLOBAL_ASYNC_HANDLER).queue_len()
}

/// Returns the current total operations capacity for the global async handler.
pub fn capacity() -> usize {
	(*GLOBAL_ASYNC_HANDLER).queue_capacity()
}

/// Evaluates whether the global async handler is active.
pub fn is_active() -> bool {
	(*GLOBAL_ASYNC_HANDLER).is_active()
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
