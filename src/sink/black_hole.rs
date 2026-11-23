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
use crate::write::IoWriter;

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

/// An `io::Empty` lookalike writing to a throwaway in-memory buffer.
struct InMemoryEmpty(Vec<u8>);

impl InMemoryEmpty {
	fn new() -> Self {
		Self(Vec::<u8>::with_capacity(DEFAULT_CAPACITY))
	}
}

impl io::Write for InMemoryEmpty {
	fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
		self.0.clear();
		self.0.extend_from_slice(bytes);
		hint::black_box(&mut self.0);
		Ok(bytes.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

/// A null log sink.
pub struct BlackHole {
	name: string::String,
	formatter: format::Formatter,
	out: IoWriter,
}

impl BlackHole {
	/// Initializes a new [`BlackHole`] log [`sink`], from a given [`BlackHoleConfig`].
	pub fn new(conf: BlackHoleConfig) -> Self {
		Self {
			name: "black hole NULL logger".into(),
			formatter: format::Formatter::new(conf.formatter_cfg),
			out: if conf.in_memory { IoWriter::new(InMemoryEmpty::new()) } else { IoWriter::new(io::empty()) },
		}
	}
}

impl sink::Sink for BlackHole {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log<'f>(&mut self, update: &sink::LogUpdate) -> io::Result<()> {
		self.formatter.write(&mut self.out, update)?;
		self.formatter.write_delimiter(&mut self.out)
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

/// Returns an initialized [`BlackHole`] log [`sink`], with default values.
pub fn default() -> BlackHole {
	BlackHole::new(BlackHoleConfig::default())
}
