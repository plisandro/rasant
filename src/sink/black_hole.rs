//! Null log [`sink`] module, intended mostly for testing.
//!
//! Black hole sinks are effectively no-op, but otherwise excercise every
//! aspect of Rasant.
use std::io;
use std::string;

use crate::attributes;
use crate::format;
use crate::sink;

/// Configuration struct for an [`BlackHole`] [`sink`].
pub struct BlackHoleConfig {
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
}

impl<'i> Default for BlackHoleConfig {
	fn default() -> Self {
		Self {
			formatter_cfg: format::FormatterConfig {
				format: format::OutputFormat::Compact,
				..format::FormatterConfig::default()
			},
		}
	}
}

/// A null log sink.
pub struct BlackHole {
	name: string::String,
	formatter: format::Formatter,
	out: io::Empty,
}

impl BlackHole {
	/// Initializes a new [`BlackHole`] log [`sink`], from a given [`BlackHoleConfig`].
	pub fn new(conf: BlackHoleConfig) -> Self {
		Self {
			name: "black hole NULL logger".into(),
			formatter: format::Formatter::new(conf.formatter_cfg),
			out: io::empty(),
		}
	}
}

impl sink::Sink for BlackHole {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		self.formatter.write(&mut self.out, update, attrs)?;
		self.formatter.write_delimiter(&mut self.out)
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

/// Returns an intitalized [`BlackHole`] log [`sink`], with default values.
pub fn default() -> BlackHole {
	BlackHole::new(BlackHoleConfig::default())
}
