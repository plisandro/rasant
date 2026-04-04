//! A [stdout](https://en.wikipedia.org/wiki/Standard_streams) [sink][`crate::sink::Sink`] module.
use std::io;

use crate::sink::format;
use crate::sink::io::{IO, IOConfig};

/// Configuration struct for an `stdout` [sink][`crate::sink::Sink`].
pub struct StdoutConfig {
	/// Name for this sink.
	pub name: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// String delimiter, inserted between log writes.
	pub delimiter: String,
	/// Whether writes to `stdout` are buffered or not.
	pub buffered: bool,
	/// Whether to flush immediately after every `stdout` write.
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

/// Initializes a `stdout` [sink][`crate::sink::Sink`] from a [`StdoutConfig`].
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

/// Returns an initialized `IO` [sink][`crate::sink::Sink`] for `stdout`, with default values.
pub fn default<'f>() -> IO<'f> {
	new(StdoutConfig::default())
}
