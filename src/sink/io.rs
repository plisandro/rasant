//! Generic I/O logging [sink] module.
//!
//! This sink can be used with any [`std::io::Write`] implementing [`Send`],
//! and supports options such as configurable delimiters between log
//! writes, flush-on-write, and buffering via [`std::io::BufWriter`].
use std::io;

use crate::constant::{IO_WRITER_DEFAULT_BUFFER_SIZE, IO_WRITER_MIN_BUFFER_SIZE};
use crate::format;
use crate::sink;
use crate::sink::Sink;
use crate::write::IoWriter;

/// Configuration struct for an [`IO`] [`sink`].
pub struct IOConfig<T: io::Write + Send> {
	/// Name for this sink.
	pub name: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether writes to the underlying [`std::io::Write`] are buffered or not, via [`std::io::BufWriter`].
	pub buffered: bool,
	/// Whether to flush immediately after every write operation.
	pub flush_on_write: bool,
	/// Whether to insert a [delimiter][`format::FormatterConfig::delimiter`] before the first log entry.
	pub initial_delimiter: bool,
	/// Whether to insert a [delimiter][`format::FormatterConfig::delimiter`] before, or after, every log write.
	pub delimiter_after_write: bool,
	/// [`io::Write`]r for this sink.
	pub out: Option<T>,
}

impl<W: io::Write + Send> Default for IOConfig<W> {
	fn default() -> Self {
		Self {
			name: String::from("default"),
			formatter_cfg: format::FormatterConfig::default(),
			buffered: true,
			flush_on_write: false,
			initial_delimiter: false,
			delimiter_after_write: false,
			out: None,
		}
	}
}

/// A [`sink`] for any implementation of [`std::io::Write`] supporting [`Send`].
pub struct IO {
	name: String,
	formatter: format::Formatter,
	written_to: bool,
	flush_on_write: bool,
	initial_delimiter: bool,
	delimiter_after_write: bool,
	out: IoWriter,
}

impl IO {
	/// Initializes a new [`IO`] [`sink`], from a given [`IOConfig`].
	pub fn new<T: io::Write + Send + 'static>(conf: IOConfig<T>) -> Self {
		let cout = match conf.out {
			Some(o) => o,
			None => panic!("missing io::Write output for I/O sink"),
		};
		let buffer_size = if conf.buffered { IO_WRITER_DEFAULT_BUFFER_SIZE } else { IO_WRITER_MIN_BUFFER_SIZE };

		Self {
			name: conf.name,
			formatter: format::Formatter::new(conf.formatter_cfg),
			written_to: false,
			initial_delimiter: conf.initial_delimiter,
			delimiter_after_write: conf.delimiter_after_write,
			flush_on_write: conf.flush_on_write,
			out: IoWriter::with_buffer_capacity(cout, buffer_size),
		}
	}
}

impl sink::Sink for IO {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate) -> io::Result<()> {
		if (!self.written_to && self.initial_delimiter) || !self.delimiter_after_write {
			self.formatter.write_delimiter(&mut self.out)?;
		}

		self.formatter.write(&mut self.out, update)?;
		if self.delimiter_after_write {
			self.formatter.write_delimiter(&mut self.out)?;
		}
		self.written_to = true;

		match self.flush_on_write {
			true => self.flush(),
			false => Ok(()),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		self.out.flush()
	}
}

impl Drop for IO {
	fn drop(&mut self) {
		if let Err(e) = self.flush() {
			panic!("failed to flush sink {name} on drop(): {e}", name = self.name);
		}
	}
}
