use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::filter::Filter;
use crate::sink::{LogUpdate, Sink};

// An Arc'ed & Mutex'ed wrapper for shared log Filter's.
pub struct FilterWrap {
	filter: Arc<Mutex<Box<dyn Filter + Send>>>,
	// TODO: add poison status
}

impl Clone for FilterWrap {
	fn clone(&self) -> Self {
		Self { filter: self.filter.clone() }
	}
}

impl FilterWrap {
	pub fn new<T: Filter + Send + 'static>(filter: T) -> Self {
		Self {
			filter: Arc::new(Mutex::new(Box::new(filter))),
		}
	}

	/*
	pub fn inner<'f>(&'f self) -> MutexGuard<'f, Box<dyn Filter + Send>> {
		self.filter.lock().unwrap()
	}
	*/

	// lightweight wrapper for Filter:pass()
	#[inline]
	pub fn pass(&self, update: &LogUpdate) -> bool {
		self.filter.lock().unwrap().pass(update)
	}

	// lightweight wrapper for Filter::skip()
	#[inline]
	pub fn skip(&self, update: &LogUpdate) -> bool {
		!self.pass(update)
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
