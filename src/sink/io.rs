//! Generic I/O logging [sink][`sink::Sink`] module.
//!
//! This sink can be used with any [`std::io::Write`] implementing [`Send`],
//! and supports options such as configurable delimiters between log
//! writes, flush-on-write, and buffering via [`std::io::BufWriter`].
use std::io;

use crate::attributes;
use crate::format;
use crate::sink;

/// Configuration struct for an [`IO`] [sink][`sink::Sink`].
pub struct IOConfig<T: io::Write + Send> {
	/// Name for this sink.
	pub name: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// String delimiter, inserted between log writes.
	pub delimiter: String,
	/// Whether writes to the underlying [`std::io::Write`] are buffered or not, via [`std::io::BufWriter`].
	pub buffered: bool,
	/// Whether to flush immediately after every write operation.
	pub flush_on_write: bool,
	/// [`io::Write`]r for this sink.
	pub out: Option<T>,
}

impl<W: io::Write + Send> Default for IOConfig<W> {
	fn default() -> Self {
		Self {
			name: String::from("default"),
			formatter_cfg: format::FormatterConfig::default(),
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
			out: None,
		}
	}
}

/// A [sink][`sink::Sink`] for any implementation of [`std::io::Write`] supporting [`Send`].
pub struct IO<'s> {
	name: String,
	formatter: format::Formatter,
	delimiter: String,
	written_to: bool,
	flush_on_write: bool,
	out: Box<dyn io::Write + Send + 's>,
}

impl<'i> IO<'i> {
	/// Initializes a new [`IO`] [sink][`sink::Sink`], from a given [`IOConfig`].
	pub fn new<T: io::Write + Send + 'i>(conf: IOConfig<T>) -> Self {
		let cout = match conf.out {
			Some(o) => o,
			None => panic!("missing io::Write output for I/O sink"),
		};
		let out: Box<dyn io::Write + Send> = if conf.buffered { Box::new(io::BufWriter::new(cout)) } else { Box::new(cout) };

		Self {
			name: conf.name,
			formatter: format::Formatter::new(conf.formatter_cfg),
			delimiter: conf.delimiter,
			written_to: false,
			flush_on_write: conf.flush_on_write,
			out: out,
		}
	}
}

impl<'i> sink::Sink for IO<'i> {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		if self.written_to {
			if let Err(e) = self.out.write(self.delimiter.as_bytes()) {
				return Err(e);
			}
		}

		self.formatter.write(&mut self.out, update, attrs)?;
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

impl Drop for IO<'_> {
	fn drop(&mut self) {
		// TODO: call self.flush() instead.
		if let Err(e) = self.out.flush() {
			panic!("failed to flush sink {name} on drop(): {e}", name = self.name);
		}
	}
}
