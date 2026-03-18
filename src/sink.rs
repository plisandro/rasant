pub mod black_hole;
pub mod file;
pub mod format;
pub mod io;
pub mod log_file;
pub mod stderr;
pub mod stdout;
pub mod string;

use std::io as std_io;

use crate::attributes;
use crate::level;
use crate::time;

pub type LogDepth = u16;
pub const MAX_LOGDEPTH: u16 = 100;

#[derive(Clone, Debug)]
pub struct LogUpdate {
	when: time::Timestamp,
	level: level::Level,
	depth: LogDepth,
	msg: String,
	attributes: attributes::Map,
}

impl LogUpdate {
	pub fn new(now: time::Timestamp, level: level::Level, depth: LogDepth, msg: String, attributes: attributes::Map) -> Self {
		Self {
			when: now,
			level: level,
			depth: depth,
			msg: msg,
			attributes: attributes,
		}
	}
}

pub trait Sink {
	fn name(&self) -> &str;
	// TODO: take ownership of LogUpdate here.
	fn log(&mut self, update: &LogUpdate) -> std_io::Result<()>;
	fn flush(&mut self) -> std_io::Result<()>;

	fn receives_all_levels(&self) -> bool {
		false
	}
}
