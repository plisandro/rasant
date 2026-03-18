use std::io;

use crate::sink::format;
use crate::sink::io::{IO, IOConfig};

pub struct StderrConfig {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
}

impl Default for StderrConfig {
	fn default() -> Self {
		Self {
			name: String::from("STDERR"),
			formatter_cfg: format::FormatterConfig::default(),
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
		}
	}
}

pub fn new<'f>(conf: StderrConfig) -> IO<'f> {
	IO::new(IOConfig {
		name: conf.name,
		formatter_cfg: conf.formatter_cfg,
		delimiter: conf.delimiter,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		out: Some(io::stderr()),
	})
}

pub fn default<'f>() -> IO<'f> {
	new(StderrConfig::default())
}
