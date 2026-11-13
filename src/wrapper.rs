use std::error;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::filter::Filter;
use crate::sink::{LogUpdate, Sink};

// A dedicated state type for wrapped filters and sinks.
#[derive(Debug)]
pub enum WrapState {
	Ok,
	Panic,
	IoError(io::Error),
}

impl Clone for WrapState {
	fn clone(&self) -> Self {
		match self {
			Self::Ok => Self::Ok,
			Self::Panic => Self::Panic,
			// horrific, but io::Error doesn't implement clone() :'(
			Self::IoError(err) => match err.raw_os_error() {
				Some(code) => Self::IoError(io::Error::from_raw_os_error(code)),
				None => Self::IoError(io::Error::new(err.kind(), err.to_string())),
			},
		}
	}
}

impl fmt::Display for WrapState {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Ok => write!(f, "Ok"),
			Self::Panic => write!(f, "implementation panicked"),
			Self::IoError(err) => write!(f, "I/O error: {err}"),
		}
	}
}

impl WrapState {
	#[inline]
	fn is_ok(&self) -> bool {
		matches!(self, Self::Ok)
	}

	#[inline]
	fn to_error(&self) -> WrapError {
		WrapError { state: self.clone() }
	}
}

// Errors associated with wrapped filters and sinks.
#[derive(Debug)]
pub struct WrapError {
	state: WrapState,
}

impl error::Error for WrapError {}

impl fmt::Display for WrapError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.state)
	}
}

// An Arc'ed & Mutex'ed wrapper for shared log Filter's.
pub struct FilterWrap {
	// Filter + sticky state.
	filter: Arc<Mutex<(Box<dyn Filter + Send>, WrapState)>>,
}

impl Clone for FilterWrap {
	fn clone(&self) -> Self {
		Self { filter: self.filter.clone() }
	}
}

impl FilterWrap {
	pub fn new<T: Filter + Send + 'static>(filter: T) -> Self {
		Self {
			filter: Arc::new(Mutex::new((Box::new(filter), WrapState::Ok))),
		}
	}

	/*
	pub fn inner<'f>(&'f self) -> &'f Box<dyn Filter + Send> {
		&(self.filter.lock().unwrap().0)
	}
	*/

	/*
	#[inline]
	fn is_poisoned(&self) -> bool {
		self.error_state.read().unwrap().is_some()
	}
	*/

	#[inline]
	fn run_with_poison_check<'f, C>(&'f self, mut callback: C) -> Result<bool, WrapError>
	where
		C: FnMut(&mut Box<dyn Filter + Send>) -> bool,
	{
		let mut filter_lock = match self.filter.lock() {
			Ok(l) => l,
			Err(_) => return Err(WrapState::Panic.to_error()),
		};
		let (filter, error_state) = &mut *filter_lock;

		if !error_state.is_ok() {
			return Err(error_state.to_error());
		}

		Ok(callback(filter))
	}

	/*
	// lightweight wrapper for Filter:pass()
	#[inline]
	pub fn pass(&self, update: &LogUpdate) -> Result<bool, WrapError> {
		self.run_with_poison_check(|f| f.pass(update))
	}
	*/

	// lightweight wrapper for Filter::skip()
	#[inline]
	pub fn skip(&self, update: &LogUpdate) -> Result<bool, WrapError> {
		self.run_with_poison_check(|f| f.skip(update))
	}
}

// An Arc'ed & Mutex'ed wrapper for shared log Sink's.
pub struct SinkWrap {
	sink: Arc<Mutex<Box<dyn Sink + Send>>>,
	// TODO: add poison status
	// TODO: add sink poison behaviour enum
}

impl Clone for SinkWrap {
	fn clone(&self) -> Self {
		Self { sink: self.sink.clone() }
	}
}

impl SinkWrap {
	pub fn new<T: Sink + Send + 'static>(sink: T) -> Self {
		Self {
			sink: Arc::new(Mutex::new(Box::new(sink))),
		}
	}

	pub fn inner<'f>(&'f self) -> MutexGuard<'f, Box<dyn Sink + Send>> {
		self.sink.lock().unwrap()
	}

	// lightweight wrapper for Sink::log()
	#[inline]
	pub fn log(&self, update: &LogUpdate) -> io::Result<()> {
		self.sink.lock().unwrap().log(update)
	}

	// lightweight wrapper for Sink::flush()
	#[inline]
	pub fn flush(&self) -> io::Result<()> {
		self.sink.lock().unwrap().flush()
	}
}
