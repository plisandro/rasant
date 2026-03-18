use std::io;

use crate::sink::format;
use crate::sink::io::{IO, IOConfig};

pub struct StdoutConfig {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
}

impl Default for StdoutConfig {
	fn default() -> Self {
		Self {
			name: String::from("STDOUT"),
			formatter_cfg: format::FormatterConfig {
				format: format::OutputFormat::ColorCompact,
				..format::FormatterConfig::default()
			},
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
		}
	}
}

pub fn new<'f>(conf: StdoutConfig) -> IO<'f> {
	IO::new(IOConfig {
		name: conf.name,
		formatter_cfg: conf.formatter_cfg,
		delimiter: conf.delimiter,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		out: Some(io::stdout()),
	})
}

pub fn default<'f>() -> IO<'f> {
	new(StdoutConfig::default())
}
