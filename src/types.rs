use ntime::Timestamp;
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

/// ShortString is a string stored in a (small) fixed-size buffer, to avoid heap allocations.
#[derive(Clone, Debug)]
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
		for i in 0..res.size {
			res.buf[i] = s.as_bytes()[i];
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

impl PartialEq for ShortString {
	fn eq(&self, other: &Self) -> bool {
		if self.size != other.size {
			return false;
		}

		for i in 0..self.size {
			if self.buf[i] != other.buf[i] {
				return false;
			}
		}

		true
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
