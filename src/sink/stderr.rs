//! A [stderr](https://en.wikipedia.org/wiki/Standard_streams) [sink][`crate::sink`] module.
use std::io;

use crate::format;
use crate::sink::io::{IO, IOConfig};

/// Configuration struct for an `stderr` [sink][`crate::sink`].
pub struct StderrConfig {
	/// Name for this sink.
	pub name: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether writes to `stderr` are buffered or not.
	pub buffered: bool,
	/// Whether to flush immediately after every `stderr` write.
	pub flush_on_write: bool,
}

impl<'i> Default for StderrConfig {
	fn default() -> Self {
		Self {
			name: String::from("STDERR"),
			formatter_cfg: format::FormatterConfig::default(),
			buffered: true,
			flush_on_write: false,
		}
	}
}

/// Initializes a `stderr` [sink][`crate::sink`] from a [`StderrConfig`].
pub fn new<'f>(conf: StderrConfig) -> IO<'f> {
	IO::new(IOConfig {
		name: conf.name,
		formatter_cfg: conf.formatter_cfg,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		out: Some(io::stderr()),
		..IOConfig::default()
	})
}

/// Returns an initialized `IO` [sink][`crate::sink`] for `stderr`, with default values.
pub fn default<'f>() -> IO<'f> {
	new(StderrConfig::default())
}
