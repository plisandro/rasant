use ntime::Timestamp;
use std::io;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::constant::SHORT_STRING_MAX_SIZE;
use crate::filter::Filter;
use crate::queue::AsyncSinkOp;
use crate::sink::Sink;

/// An Arc'ed & Mutex'ed reference to a shared log [`Filter`].
pub type FilterRef = Arc<Mutex<Box<dyn Filter + Send>>>;

/// An Arc'ed & Mutex'ed reference to a shared log [`Sink`].
pub type SinkRef = Arc<Mutex<Box<dyn Sink + Send>>>;

/// A sender channel for [`AsyncSinkOp`] async log operations.
pub type AsyncSinkSender = mpsc::Sender<AsyncSinkOp>;

/// AttributeString is a container for all string types supported as attibutes.
#[derive(Clone, Debug)]
pub struct AttributeString {
	static_buf: Option<&'static str>,
	heap_string: Option<String>,
	buf: [u8; SHORT_STRING_MAX_SIZE],
	buf_size: usize,
	needs_escaping: bool,
}

impl From<&'static str> for AttributeString {
	fn from(s: &'static str) -> Self {
		Self {
			static_buf: Some(s),
			heap_string: None,
			buf: [0; SHORT_STRING_MAX_SIZE],
			buf_size: 0,
			needs_escaping: AttributeString::has_escapable_chars(s),
		}
	}
}

impl From<String> for AttributeString {
	fn from(s: String) -> Self {
		let needs_escaping = AttributeString::has_escapable_chars(s.as_str());

		if s.len() <= SHORT_STRING_MAX_SIZE {
			// we can store this string locally \o/
			let mut res = Self {
				static_buf: None,
				heap_string: None,
				buf: [0; SHORT_STRING_MAX_SIZE],
				buf_size: s.len(),
				needs_escaping: needs_escaping,
			};
			for i in 0..res.buf_size {
				res.buf[i] = s.as_bytes()[i];
			}

			return res;
		}

		Self {
			static_buf: None,
			heap_string: Some(s),
			buf: [0; SHORT_STRING_MAX_SIZE],
			buf_size: 0,
			needs_escaping: needs_escaping,
		}
	}
}

impl<'i> AttributeString {
	/// Evaluates whether a given [`&str`] contains escapable characters.
	fn has_escapable_chars(s: &str) -> bool {
		// this is horrible, alas...
		let mut escaped_iter = s.escape_default();
		for c in s.chars() {
			match escaped_iter.next() {
				None => return true, // oops
				Some(ec) => {
					if c != ec {
						return true;
					}
				}
			}
		}
		false
	}

	/// Returns a binary length for this [`AttributeString`].
	pub fn len(&self) -> usize {
		if let Some(s) = self.static_buf {
			return s.len();
		}
		if let Some(s) = &self.heap_string {
			return s.len();
		}
		return self.buf_size;
	}

	/// Returns a [`&str`] slice for this [`AttributeString`].
	pub fn as_str(&self) -> &str {
		if let Some(s) = self.static_buf {
			return s;
		}
		if let Some(s) = &self.heap_string {
			return s.as_str();
		}
		str::from_utf8(&self.buf[0..self.buf_size]).expect("failed to deserialize AttributeString buffer")
	}

	/// lalala
	pub fn write<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		write!(out, "{}", self.as_str())
	}

	/// lalala
	pub fn write_escaped<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		match self.needs_escaping {
			false => write!(out, "{}", self.as_str()),
			true => write!(out, "{}", self.as_str().escape_default()),
		}
	}

	/// lalala
	pub fn write_quoted<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		write!(out, "\"{}\"", self.as_str())
	}

	/// lalala
	pub fn write_quoted_escaped<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		match self.needs_escaping {
			false => write!(out, "\"{}\"", self.as_str()),
			true => write!(out, "\"{}\"", self.as_str().escape_default()),
		}
	}
}

impl PartialEq for AttributeString {
	fn eq(&self, other: &Self) -> bool {
		self.as_str() == other.as_str()
	}
}

/// Rand is a 64-bit PRNG, implementing the Xorshift algorithm (https://en.wikipedia.org/wiki/Xorshift),
/// with a period of 2^64−1.
pub struct Rand {
	state: u64,
}

impl Rand {
	pub fn with_seed(seed: u64) -> Self {
		Self { state: seed }
	}

	pub fn new() -> Self {
		Self {
			state: Timestamp::now().as_nanos() as u64,
		}
	}

	pub fn next(&mut self) -> u64 {
		self.state ^= self.state << 13;
		self.state ^= self.state >> 7;
		self.state ^= self.state << 17;

		self.state
	}
}

/* ----------------------- Tests ----------------------- */

/*
#[cfg(test)]
mod short_string {
	use super::*;

	#[test]
	fn invalid() {
		assert_eq!(
			ShortString::from("this is a very long string, which surely will cause problems down the line :("),
			Err("max string size exceeded")
		);
	}

	#[test]
	fn valid() {
		// a string of length < SHORT_STRING_MAX_SIZE.
		let s = "this is a short enough string";
		let ss = ShortString::from(s).expect("ShortString initialization failed");

		assert_eq!(ss.as_str(), s);
		assert_eq!(ss.len(), s.len());

		// a string of length == SHORT_STRING_MAX_SIZE.
		let s = (0..SHORT_STRING_MAX_SIZE).map(|_| "X").collect::<String>();
		let ss = ShortString::from(&s).expect("ShortString initialization failed");

		assert_eq!(ss.as_str(), s);
		assert_eq!(ss.len(), SHORT_STRING_MAX_SIZE)
	}
}
*/

#[cfg(test)]
mod rand {
	use super::*;
	use std::array;

	#[test]
	fn generation() {
		let mut rand = Rand::with_seed(12345678);
		let got: [u64; 10] = array::from_fn(|_| rand.next());
		let want: [u64; 10] = [
			0x002f470eb7948a0c,
			0xf0a0b1ee9ea8a018,
			0x56afa382130ef758,
			0x1fda4adc73123cb6,
			0xd84256eca273f54f,
			0x69f5cfe0dba9a165,
			0x5fd04f88d2940b67,
			0x786ed9cfd23d1ab1,
			0xb98f4edcb801ecc4,
			0xb02a094b80a85e1d,
		];

		assert_eq!(got, want);
	}
}
