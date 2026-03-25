use std::io;

use crate::attributes;
use crate::sink;
use crate::sink::format;

pub struct IOConfig<T: io::Write + Send> {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
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

pub struct IO<'s> {
	name: String,
	formatter: format::Formatter,
	delimiter: String,
	flush_on_write: bool,
	out: Box<dyn io::Write + Send + 's>,
}

impl<'i> IO<'i> {
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
		self.formatter.write(&mut self.out, update, attrs)?;
		if let Err(e) = self.out.write(self.delimiter.as_bytes()) {
			return Err(e);
		}

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
