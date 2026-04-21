//! Log sinks for use with Rasant [`crate::Logger`] instances.
//!
//! This module defines the [`Sink`] and [`LogUpdate`] traits for sinks, and
//! exports all available sink types.
pub mod black_hole;
pub mod file;
pub mod io;
pub mod log_file;
pub mod stderr;
pub mod stdout;
pub mod string;

use ntime;
use std::io as std_io;

use crate::attributes;
use crate::level;

/// Depth for a [`crate::logger::Logger`] - i.e. how many parent instances it has.
pub type LogDepth = u16;

// TOOD: rework LogUpdate so it's third-party-sink friendly.
/// Encapsulates a single log update, without attributes.
#[derive(Clone, Debug)]
pub struct LogUpdate {
	/// [Timestamp][`ntime::Timestamp`] for the log update.
	pub when: ntime::Timestamp,
	/// [Level][`level::Level`] for the log update.
	pub level: level::Level,
	// TODO: use me for fancy hierarchic log output
	//depth: LogDepth,
	/// Message for the log update.
	pub msg: String,
}

impl LogUpdate {
	/// Initializes a blank placeholder [`LogUpdate`].
	pub fn blank() -> Self {
		Self {
			when: ntime::Timestamp::epoch(),
			level: level::Level::Panic,
			msg: String::from(""),
		}
	}

	/// Initializes a [`LogUpdate`] for a given timestamp, log level and log meessage.
	pub fn new(now: ntime::Timestamp, level: level::Level, msg: String) -> Self {
		Self {
			when: now,
			level: level,
			//depth: depth,
			msg: msg,
		}
	}

	/// Updates the time for a [`LogUpdate`].
	pub fn set_when(&mut self, when: ntime::Timestamp) {
		self.when = when;
	}

	/// Updates the level for a [`LogUpdate`].
	pub fn set_level(&mut self, level: level::Level) {
		self.level = level;
	}

	/// Updates the message string for a [`LogUpdate`].
	pub fn set_msg(&mut self, msg: &str) {
		self.msg.clear();
		self.msg.insert_str(0, msg);
	}
}

/// Defines a log sink usable by [Logger][`crate::logger::Logger`]s.
pub trait Sink {
	/// Returns a [`&str`] name for the sink.
	fn name(&self) -> &str;
	/// Write a [`LogUpdate`] to this sink, with associated attributes.
	fn log(&mut self, update: &LogUpdate, attrs: &attributes::Map) -> std_io::Result<()>;
	/// Flushes any pending writes for the sink.
	fn flush(&mut self) -> std_io::Result<()>;
}
