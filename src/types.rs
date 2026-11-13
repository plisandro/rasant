use std::ops;

use ntime::Timestamp;

/// A container for data structures aligned to the CPU's cache line size,
/// guaranteeing no cache thrashing on concurrently accessed items.
/// The current value should be good for all modern platforms, including ARM Cortex.
///
/// TODO: Move the (minimum) size of the CPU's cache line into a
/// HARDWARE_DESTRUCTIVE_INTERFERENCE_SIZE const, a-la-C++.
#[repr(align(128))]
pub struct CachePadded<T> {
	value: T,
}

impl<T> CachePadded<T> {
	pub fn new(v: T) -> Self {
		CachePadded { value: v }
	}
}

impl<T> ops::Deref for CachePadded<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

/// Rand is a 64-bit PRNG, implementing the Xorshift algorithm (https://en.wikipedia.org/wiki/Xorshift),
/// with a period of 2^64−1.
pub struct Rand {
	state: u64,
}

impl Rand {
	#[cfg(test)]
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
