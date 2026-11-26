//! Null I/O log [sink], intended mostly for testing.
//!
//! Black hole sinks are effectively no-op, but otherwise exercise every
//! aspect of Rasant.
use std::hint;
use std::io;
use std::string;

use crate::constant::DEFAULT_CAPACITY;
use crate::format;
use crate::sink;

/// Configuration struct for an [`BlackHole`] [`sink`].
pub struct BlackHoleConfig {
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether the sink writes into a [`io::Empty`], or a throwaway in-memory
	/// buffer. The latter is useful for benchmarking, as [`io::Empty`] can
	/// optimize the written parameters away, making runtimes not representative
	/// of real-world results.
	pub in_memory: bool,
}

impl Default for BlackHoleConfig {
	fn default() -> Self {
		Self {
			formatter_cfg: format::FormatterConfig {
				format: format::OutputFormat::Compact,
				..format::FormatterConfig::default()
			},
			in_memory: false,
		}
	}
}

/// A null log sink.
pub struct BlackHole {
	name: string::String,
	formatter: format::Formatter,
	buf: Option<Vec<u8>>,
}

impl BlackHole {
	/// Initializes a new [`BlackHole`] log [`sink`], from a given [`BlackHoleConfig`].
	pub fn new(conf: BlackHoleConfig) -> Self {
		Self {
			name: "black hole NULL logger".into(),
			formatter: format::Formatter::new(conf.formatter_cfg),
			buf: if conf.in_memory { Some(Vec::<u8>::with_capacity(DEFAULT_CAPACITY)) } else { None },
		}
	}
}

impl BlackHole {
	#[inline(always)]
	fn do_log<T: io::Write>(out: &mut T, formatter: &mut format::Formatter, update: &sink::LogUpdate) -> io::Result<()> {
		formatter.write(out, update)?;
		formatter.write_delimiter(out)
	}
}

impl sink::Sink for BlackHole {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log<'f>(&mut self, update: &sink::LogUpdate) -> io::Result<()> {
		match &mut self.buf {
			Some(b) => {
				b.clear();
				Self::do_log(b, &mut self.formatter, update)?;
				hint::black_box(&mut *b);
				Ok(())
			}
			None => Self::do_log(&mut io::empty(), &mut self.formatter, update),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

/// Returns an initialized [`BlackHole`] log [`sink`], with default values.
pub fn default() -> BlackHole {
	BlackHole::new(BlackHoleConfig::default())
}
