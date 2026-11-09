use ntime::Timestamp;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock, TryLockError};
use std::sync::{Condvar, RwLockReadGuard, WaitTimeoutResult};
use std::thread;

use crate::attributes;
use crate::constant::{DEFAULT_CAPACITY, THREAD_FINALIZE_TIMEOUT};
use crate::level::Level;
use crate::sink::{LogDepth, LogUpdate};
use crate::types::{CachePadded, SinkRef};

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
	size: CachePadded<AtomicUsize>,
	write_lock: CachePadded<RwLock<bool>>,
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
			size: CachePadded::new(AtomicUsize::new(0)),
			write_lock: CachePadded::new(RwLock::new(false)),
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
	fn lock_writes(&self) {
		*self.write_lock.write().unwrap() = true;
	}

	#[inline]
	fn unlock_writes(&self) {
		*self.write_lock.write().unwrap() = false;
	}

	#[inline]
	pub fn clear(&self) {
		self.size.store(0, Ordering::SeqCst);
	}

	fn push<'f, C>(&'f self, mut op_callback: C) -> bool
	where
		C: FnMut(&mut AsyncSinkOp),
	{
		match self.write_lock.try_read() {
			Err(e) => match e {
				// handle poisoned locks that would otherwise deadlock pushes forever
				TryLockError::Poisoned(e) => panic!("AsyncQueue write lock poisoned! {e}"),
				TryLockError::WouldBlock => false,
			},
			Ok(locked) => match *locked {
				true => false,
				false => {
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

					true
				}
			},
		}
	}

	pub fn push_flush<'f>(&'f self, sink: &SinkRef) -> bool {
		self.push(|op| op.set_flush(sink))
	}

	pub fn push_log_update<'f>(&'f self, sink: &SinkRef, update: &LogUpdate) -> bool {
		self.push(|op| op.set_log_update(sink, update))
	}
}

// A double-queue, low contention async handler for log operations.
struct AsyncHandler {
	queues: Arc<(AsyncQueue, AsyncQueue)>,
	queue_push_b: Arc<AtomicBool>,
	actuator: RwLock<Option<thread::JoinHandle<()>>>,
	closed: Arc<AtomicBool>,
	finished: Arc<(Mutex<bool>, Condvar)>,
	refcount: AtomicUsize,
}

impl AsyncHandler {
	fn new() -> Self {
		Self {
			queues: Arc::new((AsyncQueue::new(), AsyncQueue::new())),
			queue_push_b: Arc::new(AtomicBool::new(false)),
			actuator: RwLock::new(None),
			closed: Arc::new(AtomicBool::new(false)),
			finished: Arc::new((Mutex::new(false), Condvar::new())),
			refcount: AtomicUsize::new(0),
		}
	}

	fn closed(&self) -> bool {
		self.closed.load(Ordering::Relaxed)
	}

	pub fn queue_len(&self) -> usize {
		self.queues.0.len() + self.queues.1.len()
	}

	pub fn queue_capacity(&self) -> usize {
		self.queues.0.capacity() + self.queues.1.capacity()
	}

	fn start_actuator(&self) {
		let mut actuator_lock = self.actuator.write().unwrap();
		if actuator_lock.is_some() {
			return;
		}
		if self.refcount.load(Ordering::Acquire) == 0 {
			return;
		}

		self.closed.store(false, Ordering::SeqCst);
		*self.finished.0.lock().unwrap() = false;

		let queues = self.queues.clone();
		let queue_push_b = self.queue_push_b.clone();
		let closed_ref = self.closed.clone();
		let finished_ref = self.finished.clone();

		let actuator = thread::spawn(move || {
			loop {
				let (pop_queue, push_queue) = match queue_push_b.load(Ordering::Acquire) {
					false => (&queues.0, &queues.1),
					true => (&queues.1, &queues.0),
				};
				if pop_queue.is_empty() {
					match closed_ref.load(Ordering::Relaxed) {
						true => break,
						false => thread::park(),
					};
				};

				// flip the push/pop queues, and process
				push_queue.unlock_writes();
				queue_push_b.fetch_xor(true, Ordering::Release);
				pop_queue.lock_writes();

				let mut ops = AsyncQueueIter::new(pop_queue);
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

				pop_queue.clear();
			}

			// once out of the main loop, the actuator has finished
			*finished_ref.0.lock().unwrap() = true;
			finished_ref.1.notify_all();
		});

		*actuator_lock = Some(actuator);
	}

	#[inline]
	fn wake_actuator(&self) {
		match &*self.actuator.read().unwrap() {
			None => (),
			Some(t) => t.thread().unpark(),
		};
	}

	fn stop_actuator(&self) {
		let mut actuator_lock = self.actuator.write().unwrap();
		let actuator = match &*actuator_lock {
			Some(a) => a,
			None => return,
		};
		if self.refcount.load(Ordering::Acquire) != 0 {
			return;
		}

		// close this handler...
		self.closed.store(true, Ordering::SeqCst);
		actuator.thread().unpark();

		// ...and wait for the async queue actuator to complete, with a timeout. this
		// prevents potential sink write issues from locking shutdown requesters forever.
		let (finished_mutex, finished_cvar) = &*self.finished;
		let mut finished = finished_mutex.lock().unwrap();
		loop {
			if *finished {
				break;
			}

			let res: WaitTimeoutResult;
			(finished, res) = finished_cvar.wait_timeout(finished, THREAD_FINALIZE_TIMEOUT).unwrap();
			if res.timed_out() {
				break;
			}
		}

		if !*finished {
			panic!(
				"failed to shut down AsyncHandler queue after {wait:?} with {len}/{capacity} ops left",
				wait = THREAD_FINALIZE_TIMEOUT,
				len = self.queue_len(),
				capacity = self.queue_capacity(),
			);
		};

		*actuator_lock = None;
	}

	pub fn is_active(&self) -> bool {
		self.actuator.read().unwrap().is_some()
	}

	pub fn refcount(&self) -> usize {
		self.refcount.load(Ordering::Relaxed)
	}

	pub fn inc_refcount(&self) {
		if self.refcount.fetch_add(1, Ordering::AcqRel) == 0 {
			self.start_actuator();
		}
	}

	pub fn dec_refcount(&self) {
		if self.refcount.fetch_sub(1, Ordering::AcqRel) == 1 {
			self.stop_actuator();
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

	#[inline]
	fn push_queue<'f>(&'f self) -> &'f AsyncQueue {
		match self.queue_push_b.load(Ordering::SeqCst) {
			false => &self.queues.0,
			true => &self.queues.1,
		}
	}

	fn log(&self, sink: &SinkRef, update: &LogUpdate) {
		self.panic_if_closed();
		while !self.push_queue().push_log_update(sink, update) {}
		self.wake_actuator();
	}

	fn flush(&self, sink: &SinkRef) {
		self.panic_if_closed();
		while !self.push_queue().push_flush(sink) {}
		self.wake_actuator();
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
	match LazyLock::get(&GLOBAL_ASYNC_HANDLER) {
		None => 0,
		Some(h) => h.queue_len(),
	}
}

/// Returns the current total operations capacity for the global async handler.
pub fn capacity() -> usize {
	match LazyLock::get(&GLOBAL_ASYNC_HANDLER) {
		None => 0,
		Some(h) => h.queue_capacity(),
	}
}

/// Evaluates whether the global async handler is active.
pub fn is_active() -> bool {
	match LazyLock::get(&GLOBAL_ASYNC_HANDLER) {
		None => false,
		Some(h) => h.is_active(),
	}
}

/// Returns the number of active loggers referencing the global async handler.
pub fn refcount() -> usize {
	match LazyLock::get(&GLOBAL_ASYNC_HANDLER) {
		None => 0,
		Some(h) => h.refcount(),
	}
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
