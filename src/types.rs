use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::constant::SHORT_STRING_MAX_SIZE;
use crate::queue::AsyncSinkOp;
use crate::sink::Sink;

/// An Arc'ed & Mutex'ed reference to a shared log [`Sink`].
pub type SinkRef = Arc<Mutex<Box<dyn Sink + Send>>>;

/// A sender channel for [`AsyncSinkOp`] async log operations.
pub type AsyncSinkSender = mpsc::Sender<AsyncSinkOp>;

/// ShortString is a string stored in a (small) fixed-size buffer, to avoid heap allocations.
#[derive(Clone, Debug, PartialEq)]
pub struct ShortString {
	buf: [u8; SHORT_STRING_MAX_SIZE],
	size: usize,
}

impl ShortString {
	/// Initializes a [`ShortString`] from a [`&str`].
	pub fn from(s: &str) -> Result<Self, &str> {
		if s.len() > SHORT_STRING_MAX_SIZE {
			return Err("max string size exceeded");
		}

		let mut res = Self {
			buf: [0; SHORT_STRING_MAX_SIZE],
			size: s.len(),
		};
		// TODO: make me faster?
		for (dest, src) in res.buf.iter_mut().zip(s.bytes()) {
			*dest = src
		}

		Ok(res)
	}

	/// Returns the byte size for this [`ShortString`].
	pub fn len(&self) -> usize {
		self.size
	}

	/// Returns a [`&str`] slice for this [`ShortString`].
	pub fn as_str(&self) -> &str {
		str::from_utf8(&self.buf[0..self.size]).expect("failed to deserialize ShortString")
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod basic_tests {
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
