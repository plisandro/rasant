use std::io;
use std::str;

pub struct StringWriter {
	buf: io::Cursor<Vec<u8>>,
}

impl StringWriter {
	pub fn new() -> Self {
		Self { buf: io::Cursor::new(Vec::new()) }
	}

	pub fn to_string(&self) -> Result<String, str::Utf8Error> {
		match str::from_utf8(self.buf.get_ref()) {
			Ok(s) => Ok(s.to_string()),
			Err(e) => Err(e),
		}
	}
}

impl io::Write for StringWriter {
	fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
		self.buf.write(buf)
	}

	fn flush(&mut self) -> Result<(), io::Error> {
		self.buf.flush()
	}
}
