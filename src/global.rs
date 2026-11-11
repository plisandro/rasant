use std::sync::atomic::Ordering;

use crate::logger;
use crate::queue;

/// A snapshot of Rasant's global status.
/// Note that this struct describes _status_, not internal _state_; Rasant is globally stateless.
#[derive(Debug)]
pub struct Status {
	/// Total number of active [`Logger`][logger::Logger]s.
	pub loggers_count: usize,
	/// Number of [`Logger`][logger::Logger]s currently in async mode.
	pub loggers_async: usize,
	/// Whether the async queue handler thread is currently active.
	pub async_active: bool,
	/// Number of async log operations queued for processing.
	pub async_queue_size: usize,
	/// Current async log queue capacity.
	pub async_queue_capacity: usize,
}

/// Returns a snapshot of Rasant's global [`Status`].
pub fn get_status() -> Status {
	Status {
		loggers_count: logger::GLOBAL_ACTIVE_LOGGERS.load(Ordering::Relaxed),
		loggers_async: queue::refcount(),
		async_active: queue::is_active(),
		async_queue_size: queue::len(),
		async_queue_capacity: queue::capacity(),
	}
}
