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

use ntime;
use std::io as std_io;

use crate::attributes;
use crate::level;

/// [`Logger`][crate::logger::Logger] depth - i.e. how many parent instances it has.
pub type LogDepth = u16;

/// Details for a log update, _execept attributes_. This struct is later
/// encapsulaed by [`LogUpdate`], allowing to handle attributes as references
/// whenever possible, avoiding expensive copies while remaining
/// [`Sync`]-compatible.
#[derive(Clone, Debug)]
pub struct PartialLogUpdate {
	/// [Timestamp][`ntime::Timestamp`] for the log update.
	pub when: ntime::Timestamp,
	/// [Level][`level::Level`] for the log update.
	pub level: level::Level,
	// TODO: use me for fancy hierarchic log output
	//depth: LogDepth,
	/// Message for the log update.
	pub msg: String,
}

impl PartialLogUpdate {
	/// Initializes a blank placeholder [`PartialLogUpdate`].
	pub fn blank() -> Self {
		Self {
			when: ntime::Timestamp::epoch(),
			level: level::Level::Panic,
			msg: String::from(""),
		}
	}

	/// Initializes a [`PartialLogUpdate`] for a given timestamp, log level and log meessage.
	pub fn new(now: ntime::Timestamp, level: level::Level, msg: String) -> Self {
		Self {
			when: now,
			level: level,
			//depth: depth,
			msg: msg,
		}
	}

	/// Updates a [`PartialLogUpdate`] with the contents of another [`PartialLogUpdate`].
	pub fn copy_from(&mut self, other: &Self) {
		// TODO: replace with copy_from() once ntime supports it.
		self.when = other.when.clone();
		self.level = other.level;
		self.msg.clear();
		self.msg.insert_str(0, other.msg.as_str());
	}

	/// Updates the time for a [`PartialLogUpdate`].
	pub fn set_when(&mut self, when: ntime::Timestamp) {
		self.when = when;
	}

	/// Updates the level for a [`PartialLogUpdate`].
	pub fn set_level(&mut self, level: level::Level) {
		self.level = level;
	}

	/// Updates the message string for a [`PartialLogUpdate`].
	pub fn set_msg(&mut self, msg: &str) {
		self.msg.clear();
		self.msg.insert_str(0, msg);
	}
}

/// Encapsulates a full log update.
#[derive(Clone, Debug)]
pub struct LogUpdate<'s> {
	partial: &'s PartialLogUpdate,
	attrs: &'s attributes::Map,
}

impl<'i> From<(&'i PartialLogUpdate, &'i attributes::Map)> for LogUpdate<'i> {
	fn from((partial, attrs): (&'i PartialLogUpdate, &'i attributes::Map)) -> Self {
		Self { partial: partial, attrs: attrs }
	}
}

impl<'i> LogUpdate<'i> {
	/// Returns references for the underlying [`PartialLogUpdate`] + attributes map of a [`LogUpdate`].
	pub fn parts(&self) -> (&'i PartialLogUpdate, &'i attributes::Map) {
		(self.partial, self.attrs)
	}

	/// Returns the [`Timestamp`][ntime::Timestamp] for the [`LogUpdate`].
	pub fn when(&self) -> &'i ntime::Timestamp {
		&self.partial.when
	}

	/// Returne the [`Level`][level::Level] for the [`LogUpdate`].
	pub fn level(&self) -> &'i level::Level {
		&self.partial.level
	}

	/// Returne th log message for the [`LogUpdate`].
	pub fn message(&self) -> &'i str {
		self.partial.msg.as_str()
	}

	/// Returns an attributes map reference for the [`LogUpdate`].
	pub fn attributes(&self) -> &'i attributes::Map {
		self.attrs
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
