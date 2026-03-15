use std::io;

use crate::sink;
use crate::sink::format;

pub struct IOConfig<T: io::Write> {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
	pub out: Option<T>,
}

impl<W: io::Write> Default for IOConfig<W> {
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

pub struct IO {
	name: String,
	formatter: format::Formatter,
	delimiter: String,
	flush_on_write: bool,
	out: Box<dyn io::Write>,
}

impl IO {
	pub fn new<T: io::Write + 'static>(conf: IOConfig<T>) -> Self {
		let Some(cout): Option<T> = conf.out else {
			panic!("missing io::Write output for I/O sink");
		};
		let out: Box<dyn io::Write> = if conf.buffered { Box::new(io::BufWriter::new(cout)) } else { Box::new(cout) };

		Self {
			name: conf.name,
			formatter: format::Formatter::new(conf.formatter_cfg),
			delimiter: conf.delimiter,
			flush_on_write: conf.flush_on_write,
			out: out,
		}
	}
}

impl sink::Sink for IO {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate) -> io::Result<()> {
		self.formatter.write(&mut self.out, &update)?;
		if let Err(e) = self.out.write(self.delimiter.as_bytes()) {
			return Err(e);
		}
		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		self.out.flush()
	}

	fn drop(&self) {}
}
