//! Log sinks for use with Rasant [logger][crate::Logger] instances.
//!
//! This module defines the [`Sink`] and [`LogUpdate`] traits for sinks,
//! and exports all available sink types.

pub mod black_hole;
pub mod file;
pub mod io;
#[cfg(unix)]
pub mod journald;
pub mod log_file;
pub mod memory;
pub mod stderr;
pub mod stdout;
pub mod syslog;

use ntime::Timestamp;
use std::io as std_io;

use crate::attributes;
use crate::level;

/// [`Logger`][crate::logger::Logger] depth - i.e. how many parent instances it has.
pub type LogDepth = u16;

/// Encapsulates a full log update.
#[derive(Clone, Debug)]
pub struct LogUpdate<'s> {
	/// [Timestamp][`Timestamp`] for the log update.
	when: &'s Timestamp,
	/// [`Level`][level::Level] for the log update.
	level: level::Level,
	/// Number of parent instances for the [`Logger`][crate::logger::Logger] generating this log update.
	depth: LogDepth,
	/// Message for the log update.
	msg: &'s str,
	/// Attributes for the log update.
	attrs: &'s attributes::Map,
}

/// Initializes a dummy [`LogUpdate`] from a [`Timestamp`] and attributes [`Map`][attributes::Map].
impl<'i> From<(&'i Timestamp, &'i attributes::Map)> for LogUpdate<'i> {
	fn from((when, attrs): (&'i Timestamp, &'i attributes::Map)) -> Self {
		Self {
			when: when,
			level: level::Level::Trace,
			depth: 1,
			msg: "no message",
			attrs: attrs,
		}
	}
}

/// Initializes a [`LogUpdate`] from another [`LogUpdate`] and a new attributes [`Map`][attributes::Map].
impl<'i> From<(&'i LogUpdate<'i>, &'i attributes::Map)> for LogUpdate<'i> {
	fn from((other, attrs): (&'i LogUpdate, &'i attributes::Map)) -> Self {
		Self {
			when: other.when,
			level: other.level.clone(),
			depth: other.depth,
			msg: other.msg,
			attrs: attrs,
		}
	}
}

/// Initializes a [`LogUpdate`] from a full set of details.
impl<'i> From<(&'i Timestamp, level::Level, LogDepth, &'i str, &'i attributes::Map)> for LogUpdate<'i> {
	fn from((when, level, depth, msg, attrs): (&'i Timestamp, level::Level, LogDepth, &'i str, &'i attributes::Map)) -> Self {
		Self {
			when: when,
			level: level,
			depth: depth,
			msg: msg,
			attrs: attrs,
		}
	}
}

impl<'i> LogUpdate<'i> {
	/// Returns the [`Timestamp`] for the [`LogUpdate`].
	#[inline]
	pub fn when(&'i self) -> &'i Timestamp {
		self.when
	}

	/// Returns the [`Level`][level::Level] for the [`LogUpdate`].
	#[inline]
	pub fn level(&'i self) -> &'i level::Level {
		&self.level
	}

	/// Returns the [`LogDepth`] for the [`LogUpdate`].
	#[inline]
	pub fn depth(&'i self) -> &'i LogDepth {
		&self.depth
	}

	/// Returns the log message for the [`LogUpdate`].
	#[inline]
	pub fn message(&'i self) -> &'i str {
		self.msg
	}

	/// Evaluates whether the [`LogUpdate`] has any attributes defined.
	#[inline]
	pub fn no_attributes(&self) -> bool {
		self.attrs.is_empty()
	}

	/// Returns the number of attributes defined for the [`LogUpdate`].
	#[inline]
	pub fn attributes_len(&self) -> usize {
		self.attrs.len()
	}

	/// Returns wheter an attribute is present in the [`LogUpdate`].
	#[inline]
	pub fn attribute_has(&self, key: &str) -> bool {
		self.attrs.has(key)
	}

	/// Returns an attribute [`Value`][attributes::Value] and [`Metadata`][attributes::Metadata] by key from the [`LogUpdate`].
	#[inline]
	pub fn attribute_get(&'i self, key: &'i str) -> Option<(attributes::Value<'i>, attributes::Metadata)> {
		self.attrs.get(key)
	}

	/// Returns an attribute {key, [`Value`](attributes::Value)} iterator for all attributes in the [`LogUpdate`].
	#[inline]
	pub fn attribute_iter(&'i self) -> attributes::MapIter<'i> {
		self.attrs.iter()
	}

	/// Returns an attribute key iterator for all attributes in the [`LogUpdate`].
	#[inline]
	pub fn attribute_key_iter(&'i self) -> attributes::MapKeyIter<'i> {
		self.attrs.iter_key()
	}

	/// Returns an {attribute key, [`Value`](attributes::Value), [`Metadata`](attributes::Metadata)} iterator for all attributes in the [`LogUpdate`].
	#[inline]
	pub fn attribute_full_iter(&'i self) -> attributes::MapFullIter<'i> {
		self.attrs.iter_full()
	}

	/// Copies all [`LogUpdate`] attributes into a [`Map`][attributes::Map] instance.
	#[inline]
	pub fn copy_attributes_into(&'i self, attrs: &'i mut attributes::Map) {
		attrs.copy_from(self.attrs);
	}
}

impl<'i> attributes::StringIndexContainer<'i> for LogUpdate<'i> {
	#[inline]
	fn str_by_idx(&'i self, idx: usize) -> &'i str {
		self.attrs.str_by_idx(idx)
	}
}

/// Defines a log sink usable by [Logger][`crate::logger::Logger`]s.
pub trait Sink {
	/// Returns a [`&str`] name for the sink.
	fn name(&self) -> &str;
	/// Write a [`LogUpdate`] to this sink, with associated attributes.
	fn log<'f>(&mut self, update: &'f LogUpdate) -> std_io::Result<()>;
	/// Flushes any pending writes for the sink.
	fn flush(&mut self) -> std_io::Result<()>;
}
