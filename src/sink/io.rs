use std::io;

use crate::sink;
use crate::sink::format;

pub struct IOConfig<W: io::Write> {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
	pub writer: Option<W>,
}

impl<W: io::Write> Default for IOConfig<W> {
	fn default() -> Self {
		Self {
			name: String::from("default"),
			formatter_cfg: format::FormatterConfig::default(),
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
			writer: None,
		}
	}
}

pub struct IO {
	name: String,
	formatter: format::Formatter,
	delimiter: String,
	flush_on_write: bool,
	writer: Box<dyn io::Write>,
}

impl IO {
	pub fn new<T: io::Write + 'static>(conf: IOConfig<T>) -> Self {
		let Some(cwriter): Option<T> = conf.writer else {
			panic!("missing io::Write output for I/O sink");
		};
		let writer: Box<dyn io::Write> = if conf.buffered { Box::new(io::BufWriter::new(cwriter)) } else { Box::new(cwriter) };

		Self {
			name: conf.name,
			formatter: format::Formatter::new(conf.formatter_cfg),
			delimiter: conf.delimiter,
			flush_on_write: conf.flush_on_write,
			writer: writer,
		}
	}
}

impl sink::Sink for IO {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn write(&mut self, update: &sink::LogUpdate) {
		let mut out = self.formatter.format(&update);
		out.push_str(&self.delimiter);

		match self.writer.write(out.as_bytes()) {
			Err(e) => panic!("failed to write to log sink {name}: {e:?}", name = self.name, e = e),
			Ok(_) => {}
		}
		if self.flush_on_write {
			self.flush();
		}
	}

	fn flush(&mut self) {
		match self.writer.flush() {
			Err(e) => panic!("failed to flush log sink {name}: {e:?}", name = self.name, e = e),
			Ok(_) => {}
		}
	}

	fn drop(&self) {}
}
