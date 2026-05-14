use ntime::Timestamp;
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

/// A escape function converting [`chars`] into [`u8`]s.
pub type StringEscapeFn<'t> = Option<fn(char) -> &'t [u8]>;

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
