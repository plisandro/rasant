pub mod black_hole;
pub mod file;
pub mod format;
pub mod io;
pub mod log_file;
pub mod stderr;
pub mod stdout;
pub mod string;

use ntime;
use std::io as std_io;

use crate::attributes;
use crate::level;

pub type LogDepth = u16;
pub const MAX_LOGDEPTH: u16 = 100;

#[derive(Clone, Debug)]
pub struct LogUpdate {
	when: ntime::Timestamp,
	level: level::Level,
	// TODO: use me for fancy hierarchic log output
	//depth: LogDepth,
	msg: String,
}

impl LogUpdate {
	pub fn new(now: ntime::Timestamp, level: level::Level, msg: String) -> Self {
		Self {
			when: now,
			level: level,
			//depth: depth,
			msg: msg,
		}
	}
}

pub trait Sink {
	fn name(&self) -> &str;
	fn log(&mut self, update: &LogUpdate, attrs: &attributes::Map) -> std_io::Result<()>;
	fn flush(&mut self) -> std_io::Result<()>;

	fn receives_all_levels(&self) -> bool {
		false
	}
}
