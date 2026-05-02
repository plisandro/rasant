use ntime::Timestamp;
use std::io;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::filter::Filter;
use crate::queue::AsyncSinkOp;
use crate::sink::Sink;

/// An Arc'ed & Mutex'ed reference to a shared log [`Filter`].
pub type FilterRef = Arc<Mutex<Box<dyn Filter + Send>>>;

/// An Arc'ed & Mutex'ed reference to a shared log [`Sink`].
pub type SinkRef = Arc<Mutex<Box<dyn Sink + Send>>>;

/// A sender channel for [`AsyncSinkOp`] async log operations.
pub type AsyncSinkSender = mpsc::Sender<AsyncSinkOp>;

/// A string container for any of the types supported in attributes: [`String`], static [`&str`], or an index within a string container.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeString {
	heap_string: Option<String>,
	static_buf: Option<&'static str>,
	idx: usize,
	needs_escaping: bool,
}

/// Defines a trait to resolve indexed [`AttributeString`]s from a container.
pub trait AttributeStringSeek {
	/// Resolves an indexed [`AttributeString`] to [`&str`].
	fn str_seek<'f>(&'f self, idx: usize) -> &'f str;
}

impl From<String> for AttributeString {
	fn from(s: String) -> Self {
		let needs_escaping = Self::has_escapable_chars(s.as_str());
		Self {
			heap_string: Some(s),
			static_buf: None,
			idx: 0,
			needs_escaping: needs_escaping,
		}
	}
}

impl From<&'static str> for AttributeString {
	fn from(s: &'static str) -> Self {
		Self {
			heap_string: None,
			static_buf: Some(s),
			idx: 0,
			needs_escaping: Self::has_escapable_chars(s),
		}
	}
}

impl From<(usize, bool)> for AttributeString {
	fn from(args: (usize, bool)) -> Self {
		let (idx, needs_escaping) = args;
		Self {
			heap_string: None,
			static_buf: None,
			idx: idx,
			needs_escaping: needs_escaping,
		}
	}
}

impl<'i> AttributeString {
	/// Evaluates whether a source string needs escaping.
	fn has_escapable_chars(s: &str) -> bool {
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

	/// Checks if this [`AttributeString`] requires escaping.
	pub fn needs_escaping(&self) -> bool {
		self.needs_escaping
	}

	/// Returns a [`&str`] for this [`AttributeString`], if it's heap-stored.
	pub fn as_heap_str(&'i self) -> Option<&'i str> {
		match &self.heap_string {
			Some(s) => Some(s.as_str()),
			None => None,
		}
	}

	/// Casts a [`AttributeString`] to a [`&str`], resolving indexed strings via [`AttributeStringSeek`] when necessary.
	pub fn as_str<S: AttributeStringSeek>(&'i self, seeker: &'i S) -> &'i str {
		if let Some(s) = &self.heap_string {
			return s.as_str();
		}
		if let Some(s) = self.static_buf {
			return s;
		}

		seeker.str_seek(self.idx)
	}

	/// Returns the container index for this [`AttributeString], if any.
	pub fn idx(&self) -> Option<usize> {
		match self.heap_string.is_some() || self.static_buf.is_some() {
			false => Some(self.idx),
			true => None,
		}
	}

	/// Creates an indexed [`AttributeString`] copy, preserving original settings.
	pub fn to_indexed(&self, idx: usize) -> Self {
		Self {
			heap_string: None,
			static_buf: None,
			idx: idx,
			needs_escaping: self.needs_escaping,
		}
	}

	/// Re-aligns string container indeces for an [`AttributeString`], given a deleted index.
	pub fn realign_by_deleted_idx(&mut self, deleted_idx: usize) {
		if self.heap_string.is_some() || self.static_buf.is_some() {
			return;
		}
		if self.idx != 0 && self.idx >= deleted_idx {
			self.idx -= 1;
		}
	}

	/// Writes an [`AttributeString`] into a [`io::Write`].
	pub fn write<O: io::Write, S: AttributeStringSeek>(&self, out: &mut O, seeker: &'i S) -> io::Result<()> {
		write!(out, "{}", self.as_str(seeker))
	}

	/// Writes an [`AttributeString`] into a [`io::Write`], escaping characters when needed.
	pub fn write_escaped<O: io::Write, S: AttributeStringSeek>(&self, out: &mut O, seeker: &'i S) -> io::Result<()> {
		match self.needs_escaping {
			false => write!(out, "{}", self.as_str(seeker)),
			true => write!(out, "{}", self.as_str(seeker).escape_default()),
		}
	}

	/// Writes an [`AttributeString`] into a [`io::Write`], between quotes.
	pub fn write_quoted<O: io::Write, S: AttributeStringSeek>(&self, out: &mut O, seeker: &'i S) -> io::Result<()> {
		write!(out, "\"{}\"", self.as_str(seeker))
	}

	/// Writes an [`AttributeString`] into a [`io::Write`], between quotes, and escaping characters when needed.
	pub fn write_quoted_escaped<O: io::Write, S: AttributeStringSeek>(&self, out: &mut O, seeker: &'i S) -> io::Result<()> {
		match self.needs_escaping {
			false => write!(out, "\"{}\"", self.as_str(seeker)),
			true => write!(out, "\"{}\"", self.as_str(seeker).escape_default()),
		}
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

#[cfg(test)]
mod attribute_string {
	use super::*;

	struct DummySeeker {}
	impl AttributeStringSeek for DummySeeker {
		fn str_seek<'f>(&'f self, _: usize) -> &'f str {
			"indexed string"
		}
	}

	#[test]
	fn slice() {
		let s = AttributeString::from("static slice");
		let seeker = DummySeeker {};

		assert_eq!(s.as_heap_str(), None);
		assert_eq!(s.as_str(&seeker), "static slice");
		assert_eq!(s.idx(), None);
		assert_eq!(s.needs_escaping(), false);
	}

	#[test]
	fn string() {
		let s = AttributeString::from(String::from("heap string"));
		let seeker = DummySeeker {};

		assert_eq!(s.as_heap_str(), Some("heap string"));
		assert_eq!(s.as_str(&seeker), "heap string");
		assert_eq!(s.idx(), None);
		assert_eq!(s.needs_escaping(), false);
	}

	#[test]
	fn indexed() {
		let mut s = AttributeString::from((1, false));
		let seeker = DummySeeker {};

		assert_eq!(s.as_heap_str(), None);
		assert_eq!(s.as_str(&seeker), "indexed string");
		assert_eq!(s.idx(), Some(1));

		s.realign_by_deleted_idx(3);
		assert_eq!(s.idx(), Some(1));
		s.realign_by_deleted_idx(0);
		assert_eq!(s.idx(), Some(0));
		s.realign_by_deleted_idx(0);
		assert_eq!(s.idx(), Some(0));
	}

	#[test]
	fn escaping() {
		let s = AttributeString::from("declaró\nen\tcontra");
		let seeker = DummySeeker {};

		assert_eq!(s.as_heap_str(), None);
		assert_eq!(s.as_str(&seeker), "declaró\nen\tcontra");
		assert_eq!(s.idx(), None);
		assert_eq!(s.needs_escaping(), true);

		let mut out: Vec<u8> = Vec::new();
		s.write(&mut out, &seeker).unwrap();
		assert_eq!(str::from_utf8(&out).unwrap(), "declaró\nen\tcontra");

		out.clear();
		s.write_quoted(&mut out, &seeker).unwrap();
		assert_eq!(str::from_utf8(&out).unwrap(), "\"declaró\nen\tcontra\"");

		out.clear();
		s.write_escaped(&mut out, &seeker).unwrap();
		assert_eq!(str::from_utf8(&out).unwrap(), "declar\\u{f3}\\nen\\tcontra");

		out.clear();
		s.write_quoted_escaped(&mut out, &seeker).unwrap();
		assert_eq!(str::from_utf8(&out).unwrap(), "\"declar\\u{f3}\\nen\\tcontra\"");
	}
}

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
