use std::any::Any;
use std::io;
use std::io::Write;
#[cfg(test)]
use std::str;

use crate::constant::{DEFAULT_CAPACITY, IO_WRITER_MIN_BUFFER_SIZE};

const STR_B10_SINGLE_DIGIT: &[u8; 10] = b"0123456789";
const STR_B10_DOUBLE_DIGIT: &[&[u8; 2]; 100] = &[
	b"00", b"01", b"02", b"03", b"04", b"05", b"06", b"07", b"08", b"09", b"10", b"11", b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22", b"23", b"24", b"25", b"26", b"27",
	b"28", b"29", b"30", b"31", b"32", b"33", b"34", b"35", b"36", b"37", b"38", b"39", b"40", b"41", b"42", b"43", b"44", b"45", b"46", b"47", b"48", b"49", b"50", b"51", b"52", b"53", b"54", b"55",
	b"56", b"57", b"58", b"59", b"60", b"61", b"62", b"63", b"64", b"65", b"66", b"67", b"68", b"69", b"70", b"71", b"72", b"73", b"74", b"75", b"76", b"77", b"78", b"79", b"80", b"81", b"82", b"83",
	b"84", b"85", b"86", b"87", b"88", b"89", b"90", b"91", b"92", b"93", b"94", b"95", b"96", b"97", b"98", b"99",
];

const STR_B16_HEADER: &[u8; 2] = b"0x";
const STR_B16_SINGLE_DIGIT: &[u8; 16] = b"0123456789abcdef";
const STR_B16_DOUBLE_DIGIT: &[&[u8; 2]; 256] = &[
	b"00", b"01", b"02", b"03", b"04", b"05", b"06", b"07", b"08", b"09", b"0a", b"0b", b"0c", b"0d", b"0e", b"0f", b"10", b"11", b"12", b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"1a", b"1b",
	b"1c", b"1d", b"1e", b"1f", b"20", b"21", b"22", b"23", b"24", b"25", b"26", b"27", b"28", b"29", b"2a", b"2b", b"2c", b"2d", b"2e", b"2f", b"30", b"31", b"32", b"33", b"34", b"35", b"36", b"37",
	b"38", b"39", b"3a", b"3b", b"3c", b"3d", b"3e", b"3f", b"40", b"41", b"42", b"43", b"44", b"45", b"46", b"47", b"48", b"49", b"4a", b"4b", b"4c", b"4d", b"4e", b"4f", b"50", b"51", b"52", b"53",
	b"54", b"55", b"56", b"57", b"58", b"59", b"5a", b"5b", b"5c", b"5d", b"5e", b"5f", b"60", b"61", b"62", b"63", b"64", b"65", b"66", b"67", b"68", b"69", b"6a", b"6b", b"6c", b"6d", b"6e", b"6f",
	b"70", b"71", b"72", b"73", b"74", b"75", b"76", b"77", b"78", b"79", b"7a", b"7b", b"7c", b"7d", b"7e", b"7f", b"80", b"81", b"82", b"83", b"84", b"85", b"86", b"87", b"88", b"89", b"8a", b"8b",
	b"8c", b"8d", b"8e", b"8f", b"90", b"91", b"92", b"93", b"94", b"95", b"96", b"97", b"98", b"99", b"9a", b"9b", b"9c", b"9d", b"9e", b"9f", b"a0", b"a1", b"a2", b"a3", b"a4", b"a5", b"a6", b"a7",
	b"a8", b"a9", b"aa", b"ab", b"ac", b"ad", b"ae", b"af", b"b0", b"b1", b"b2", b"b3", b"b4", b"b5", b"b6", b"b7", b"b8", b"b9", b"ba", b"bb", b"bc", b"bd", b"be", b"bf", b"c0", b"c1", b"c2", b"c3",
	b"c4", b"c5", b"c6", b"c7", b"c8", b"c9", b"ca", b"cb", b"cc", b"cd", b"ce", b"cf", b"d0", b"d1", b"d2", b"d3", b"d4", b"d5", b"d6", b"d7", b"d8", b"d9", b"da", b"db", b"dc", b"dd", b"de", b"df",
	b"e0", b"e1", b"e2", b"e3", b"e4", b"e5", b"e6", b"e7", b"e8", b"e9", b"ea", b"eb", b"ec", b"ed", b"ee", b"ef", b"f0", b"f1", b"f2", b"f3", b"f4", b"f5", b"f6", b"f7", b"f8", b"f9", b"fa", b"fb",
	b"fc", b"fd", b"fe", b"ff",
];

/// A valid [`io::Writer`] sink for [`IoWriter`] - not to be confused with general log sinks.
trait IoWriterSink: io::Write + Send + Any {}
impl<W: io::Write + Send + 'static> IoWriterSink for W {}

/// A ['io::Write`] wrapper, with basic formatting and buffereing for small operations.
pub struct IoWriter {
	out: Box<dyn IoWriterSink>,
	buf: Box<[u8]>,
	buf_idx: usize,
}

/// [`IoWriter`] implement the [`io::Write`] interface for convenience.
impl io::Write for IoWriter {
	fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
		self.bytes(bytes)?;
		Ok(bytes.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		IoWriter::flush(self)
	}
}

impl IoWriter {
	/// Initializes a [`IoWriter`] with a given buffer capacity, with a minimum of [`IO_WRITER_MIN_BUFFER_SIZE`].
	pub fn with_buffer_capacity<T: io::Write + Send + 'static>(out: T, buf_capacity: usize) -> Self {
		let buf_capacity = buf_capacity.max(IO_WRITER_MIN_BUFFER_SIZE);

		Self {
			out: Box::new(out),
			buf: vec![0_u8; buf_capacity].into_boxed_slice(),
			buf_idx: 0,
		}
	}

	/// Initializes a [`IoWriter`] from a generic [`io::Write`], with minimum buffer capacity.
	#[inline]
	pub fn new<T: io::Write + Send + 'static>(out: T) -> Self {
		Self::with_buffer_capacity(out, IO_WRITER_MIN_BUFFER_SIZE)
	}

	/// Initializes a [`IoWriter`] from a ['Vec<u8>'], to be later accessed via `into_*()`.
	#[inline]
	pub fn new_bytes() -> Self {
		Self::new(Vec::<u8>::with_capacity(DEFAULT_CAPACITY))
	}

	/// Consumes the writer and returns its contents as `Vec<u8>`, if possible.
	#[inline]
	pub fn inner<T: io::Write + Send + 'static>(mut self) -> io::Result<T> {
		self.flush()?;
		let any: Box<dyn Any + Send> = self.out;
		any.downcast::<T>().map(|b| *b).map_err(|_| io::Error::other("IoWriter inner type mismatch"))
	}

	/// Consumes the writer and returns its contents as [`Vec<u8>`], if possible.
	#[inline]
	pub fn into_bytes(self) -> io::Result<Vec<u8>> {
		self.inner::<Vec<u8>>()
	}

	/// Consumes the writer and returns its contents as a [`String`], if possible.
	#[inline]
	pub fn into_string(self) -> io::Result<String> {
		let bytes = self.into_bytes()?;
		String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}

	/// Flushes all buffer pending contents.
	#[inline(always)]
	pub fn flush_buffer(&mut self) -> io::Result<()> {
		self.out.write_all(&self.buf[0..self.buf_idx])?;
		self.buf_idx = 0;
		Ok(())
	}

	/// Flushes all pending writes on the output sink.
	#[inline(always)]
	pub fn flush(&mut self) -> io::Result<()> {
		self.flush_buffer()?;
		self.out.flush()
	}

	/// Returns the remaning buffer capacity.
	#[inline(always)]
	fn buffer_available(&mut self) -> usize {
		self.buf.len() - self.buf_idx
	}

	/// Checks if the buffer is full.
	#[inline(always)]
	fn buffer_full(&mut self) -> bool {
		self.buf_idx >= self.buf.len()
	}

	/// Writes a single [`u8`] into the output.
	#[inline]
	pub fn u8(&mut self, c: u8) -> io::Result<()> {
		if self.buffer_full() {
			self.flush_buffer()?
		}
		self.buf[self.buf_idx] = c;
		self.buf_idx += 1;

		Ok(())
	}

	/// Writes repeated number of [`u8`]s into the output.
	#[inline]
	pub fn u8_repeated(&mut self, c: u8, count: usize) -> io::Result<()> {
		let mut count = count;

		while count > 0 {
			if self.buffer_full() {
				self.flush_buffer()?;
			}
			let n = count.min(self.buffer_available());
			self.buf[self.buf_idx..self.buf_idx + n].fill(c);
			self.buf_idx += n;
			count -= n;
		}

		Ok(())
	}

	/// Writes a space (` `) into the output.
	#[inline(always)]
	pub fn space(&mut self) -> io::Result<()> {
		self.u8(b' ')
	}

	/// Writes a `&[u8]` into the output.
	#[inline]
	pub fn bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
		let bytes_len = bytes.len();

		if bytes_len <= self.buffer_available() {
			self.buf[self.buf_idx..self.buf_idx + bytes_len].copy_from_slice(&bytes[0..bytes_len]);
			self.buf_idx += bytes_len;
			return Ok(());
		}

		self.flush_buffer()?;
		self.out.write_all(bytes)
	}

	/// Writes a [&str] into the output.
	#[inline]
	pub fn str(&mut self, s: &str) -> io::Result<()> {
		self.bytes(s.as_bytes())
	}

	/// Writes a right-padded [&str] into the output.
	#[inline]
	pub fn str_right_padded(&mut self, s: &str, pad: u8, width: usize) -> io::Result<()> {
		self.bytes(s.as_bytes())?;

		if width > s.len() {
			self.u8_repeated(pad, width - s.len())?;
		}

		Ok(())
	}

	/// Writes a [bool] as string (`true`/`false`) into the output..
	#[inline]
	pub fn bool(&mut self, b: &bool) -> io::Result<()> {
		self.str(match *b {
			true => "true",
			false => "false",
		})
	}

	/// Computes the number of decimal digits for a [`u64`].
	#[inline(always)]
	fn u64_dec_digits(n: &u64) -> usize {
		(*n | 1).ilog10() as usize + 1
	}

	/// Computes the number of hex digits for a [`u128`].
	#[inline(always)]
	fn u128_hex_digits(n: &u128) -> usize {
		((*n | 1).ilog2() / 4) as usize + 1
	}

	/// Writes a [u64] as a decimal string into the output.
	#[inline]
	pub fn dec_u64(&mut self, n: &u64) -> io::Result<()> {
		let digits = Self::u64_dec_digits(n);
		if digits > self.buffer_available() {
			self.flush_buffer()?
		}

		match digits {
			1 => {
				// fast implementations for most commonly used number types; single digit...
				self.buf[self.buf_idx] = STR_B10_SINGLE_DIGIT[*n as usize];
				self.buf_idx += 1;
			}
			2 => {
				// ...double-digit...
				self.buf[self.buf_idx..self.buf_idx + 2].copy_from_slice(STR_B10_DOUBLE_DIGIT[*n as usize]);
				self.buf_idx += 2;
			}
			3 => {
				// ...3 digits...
				let high = (n / 100) as usize;
				let low = (n % 100) as usize;
				self.buf[self.buf_idx] = STR_B10_SINGLE_DIGIT[high];
				self.buf[self.buf_idx + 1..self.buf_idx + 3].copy_from_slice(STR_B10_DOUBLE_DIGIT[low]);
				self.buf_idx += 3;
			}
			4 => {
				// ...and 4 digits.
				let high = (n / 100) as usize;
				let low = (n % 100) as usize;
				self.buf[self.buf_idx..self.buf_idx + 2].copy_from_slice(STR_B10_DOUBLE_DIGIT[high]);
				self.buf[self.buf_idx + 2..self.buf_idx + 4].copy_from_slice(STR_B10_DOUBLE_DIGIT[low]);
				self.buf_idx += 4;
			}
			_ => {
				// iterative output for everything else
				let mut n = *n;
				let mut rev_idx = self.buf_idx + digits;

				while n >= 100 {
					let pair = (n % 100) as usize;
					n /= 100;
					self.buf[rev_idx - 2..rev_idx].copy_from_slice(STR_B10_DOUBLE_DIGIT[pair]);
					rev_idx -= 2;
				}
				match n {
					0..10 => self.buf[rev_idx - 1] = STR_B10_SINGLE_DIGIT[n as usize],
					_ => self.buf[rev_idx - 2..rev_idx].copy_from_slice(STR_B10_DOUBLE_DIGIT[n as usize]),
				}

				self.buf_idx += digits;
			}
		}

		Ok(())
	}

	/// Writes a [u64] as a decimal string into the output, with configurable padding.
	#[inline]
	pub fn dec_u64_padded(&mut self, n: &u64, pad_width: usize) -> io::Result<()> {
		let digits = Self::u64_dec_digits(n);
		if pad_width > digits {
			self.u8_repeated(b'0', pad_width - digits)?;
		}
		self.dec_u64(n)
	}

	/// Writes a [i64] as a decimal string into the output.
	#[inline]
	pub fn dec_i64(&mut self, n: &i64) -> io::Result<()> {
		if *n < 0 {
			self.u8(b'-')?;
		}
		self.dec_u64(&n.unsigned_abs())
	}

	/// Writes a [u128] as a decimal string into the output.
	pub fn dec_u128(&mut self, n: &u128) -> io::Result<()> {
		match u64::try_from(*n) {
			Ok(n) => self.dec_u64(&n),
			Err(_) => {
				// yikes, an actual 128-bit uint :'( split the number every 15
				// digits (max 39 for a decimal u128), and serialize in chunks.
				const DIGITS_SPLIT: usize = 15;
				const TEN_15: u128 = 1_000_000_000_000_000;

				let (high, low) = (n / TEN_15, n % TEN_15);
				let (high, mid) = (high / TEN_15, high % TEN_15);

				if high != 0 {
					self.dec_u64(&(high as u64))?;
					self.dec_u64_padded(&(mid as u64), DIGITS_SPLIT)?;
					self.dec_u64_padded(&(low as u64), DIGITS_SPLIT)
				} else if mid != 0 {
					self.dec_u64(&(mid as u64))?;
					self.dec_u64_padded(&(low as u64), DIGITS_SPLIT)
				} else {
					self.dec_u64(&(low as u64))
				}
			}
		}
	}

	/// Writes a [i128] as a decimal string into the output.
	#[inline(always)]
	pub fn dec_i128(&mut self, n: &i128) -> io::Result<()> {
		if *n < 0 {
			self.u8(b'-')?;
		}
		self.dec_u128(&n.unsigned_abs())
	}

	/// Writes a [u128] as a hex string into the output.
	#[inline]
	pub fn hex_u128(&mut self, n: &u128) -> io::Result<()> {
		let digits = Self::u128_hex_digits(n);
		if digits + STR_B16_HEADER.len() > self.buffer_available() {
			self.flush_buffer()?;
		}

		self.bytes(STR_B16_HEADER)?;

		match digits {
			1 => {
				// fast implementations for most commonly used number types; single digit...
				self.buf[self.buf_idx] = STR_B16_SINGLE_DIGIT[*n as usize];
				self.buf_idx += 1;
			}
			2 => {
				// ...double-digit...
				self.buf[self.buf_idx..self.buf_idx + 2].copy_from_slice(STR_B16_DOUBLE_DIGIT[*n as usize]);
				self.buf_idx += 2;
			}
			3 => {
				// ...3 digits...
				self.buf[self.buf_idx + 1..self.buf_idx + 3].copy_from_slice(STR_B16_DOUBLE_DIGIT[(n & 0xff) as usize]);
				self.buf[self.buf_idx] = STR_B16_SINGLE_DIGIT[(n >> 8) as usize];
				self.buf_idx += 3;
			}
			4 => {
				// ...and 4 digits.
				self.buf[self.buf_idx + 2..self.buf_idx + 4].copy_from_slice(STR_B16_DOUBLE_DIGIT[(n & 0xff) as usize]);
				self.buf[self.buf_idx..self.buf_idx + 2].copy_from_slice(STR_B16_DOUBLE_DIGIT[(n >> 8) as usize]);
				self.buf_idx += 4;
			}
			_ => {
				// iterative output for everything else
				let mut n = *n;
				let mut rev_idx = self.buf_idx + digits;

				while n > 0xff {
					self.buf[rev_idx - 2..rev_idx].copy_from_slice(STR_B16_DOUBLE_DIGIT[(n & 0xff) as usize]);
					n >>= 8;
					rev_idx -= 2;
				}
				match rev_idx - self.buf_idx {
					1 => self.buf[rev_idx - 1] = STR_B16_SINGLE_DIGIT[n as usize],
					2 => self.buf[rev_idx - 2..rev_idx].copy_from_slice(STR_B16_DOUBLE_DIGIT[n as usize]),
					_ => (),
				}

				self.buf_idx += digits;
			}
		}

		Ok(())
	}

	/// Writes a [i128] as a hex string into the output.
	#[inline]
	pub fn hex_i128(&mut self, n: &i128) -> io::Result<()> {
		if *n < 0 {
			self.u8(b'-')?;
		}
		self.hex_u128(&n.unsigned_abs())
	}

	/// Writes a [usize] as a decimal string into the output.
	#[inline]
	pub fn dec_usize(&mut self, n: &usize) -> io::Result<()> {
		self.dec_u128(&(*n as u128))
	}

	/// Writes a [isize] as a decimal string into the output.
	#[inline]
	pub fn dec_isize(&mut self, n: &isize) -> io::Result<()> {
		self.dec_i128(&(*n as i128))
	}

	/// Writes a [usize] as a hex string into the output.
	#[inline]
	pub fn hex_usize(&mut self, n: &usize) -> io::Result<()> {
		self.hex_u128(&(*n as u128))
	}

	/// Writes a [isize] as a hex string into the output.
	#[inline]
	pub fn hex_isize(&mut self, n: &isize) -> io::Result<()> {
		self.hex_i128(&(*n as i128))
	}

	/// Writes a [f64] as a decimal string into the output.
	// TODO: optimize me!
	#[inline]
	pub fn dec_f64(&mut self, n: &f64) -> io::Result<()> {
		self.flush_buffer()?;
		write!(self.out, "{}", n)
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn basic() {
		let mut out = IoWriter::new_bytes();

		out.u8(b'a').unwrap();
		out.u8(b'B').unwrap();
		out.u8(b'c').unwrap();
		out.u8(b'D').unwrap();
		out.space().unwrap();
		out.str("lalalala").unwrap();
		out.space().unwrap();
		out.u8_repeated(b'X', 5).unwrap();
		out.space().unwrap();
		out.bool(&false).unwrap();
		out.space().unwrap();
		out.bool(&true).unwrap();

		assert_eq!(out.into_string().unwrap(), "aBcD lalalala XXXXX false true");
	}

	#[test]
	fn string() {
		let mut out = IoWriter::new_bytes();

		out.str("lalala").unwrap();
		out.space().unwrap();
		out.str("lololo").unwrap();
		out.space().unwrap();
		out.str_right_padded("right_pad_ok", b'_', 16).unwrap();
		out.space().unwrap();
		out.str_right_padded("right_pad_nok", b' ', 5).unwrap();
		out.space().unwrap();
		out.str_right_padded("right_pad_exact", b'X', 15).unwrap();

		assert_eq!(out.into_string().unwrap(), "lalala lololo right_pad_ok____ right_pad_nok right_pad_exact");
	}

	#[test]
	fn dec_u64() {
		for (n, want) in [
			(0, "0"),
			(9, "9"),
			(10, "10"),
			(999, "999"),
			(1000, "1000"),
			(2026, "2026"),
			(9999, "9999"),
			(10000, "10000"),
			(9383766512304, "9383766512304"),
			(u64::MAX, "18446744073709551615"),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_u64(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn dec_i64() {
		for (n, want) in [
			(0, "0"),
			(-9, "-9"),
			(10, "10"),
			(-999, "-999"),
			(1000, "1000"),
			(-2026, "-2026"),
			(9999, "9999"),
			(-10000, "-10000"),
			(i64::MAX, "9223372036854775807"),
			(i64::MIN, "-9223372036854775808"),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_i64(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn dec_u64_padded() {
		for (n, pad_width, want) in [
			(0, 0, "0"),
			(0, 1, "0"),
			(0, 2, "00"),
			(5, 2, "05"),
			(5, 6, "000005"),
			(99, 1, "99"),
			(99, 2, "99"),
			(99, 3, "099"),
			(9383766512304, 5, "9383766512304"),
			(9383766512304, 13, "9383766512304"),
			(9383766512304, 16, "0009383766512304"),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_u64_padded(&n, pad_width).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn dec_u128() {
		for (n, want) in [
			(0, "0"),
			(9, "9"),
			(999, "999"),
			(2026, "2026"),
			(9383766512304, "9383766512304"),
			(u64::MAX as u128, "18446744073709551615"),
			(u128::MAX, "340282366920938463463374607431768211455"),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_u128(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn dec_i128() {
		for (n, want) in [
			(0, "0"),
			(-9, "-9"),
			(999, "999"),
			(-2026, "-2026"),
			(9383766512304, "9383766512304"),
			(i64::MAX as i128, "9223372036854775807"),
			(i64::MIN as i128, "-9223372036854775808"),
			(i128::MAX, "170141183460469231731687303715884105727"),
			(i128::MIN, "-170141183460469231731687303715884105728"),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_i128(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn hex_u128() {
		for (n, want) in [
			(0, "0x0"),
			(9, "0x9"),
			(10, "0xa"),
			(11, "0xb"),
			(999, "0x3e7"),
			(2026, "0x7ea"),
			(10000, "0x2710"),
			(9383766512304, "0x888d411d2b0"),
			(u64::MAX as u128, "0xffffffffffffffff"),
			(u128::MAX, "0xffffffffffffffffffffffffffffffff"),
		] {
			let mut out = IoWriter::new_bytes();

			out.hex_u128(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn hex_i128() {
		for (n, want) in [
			(0, "0x0"),
			(-9, "-0x9"),
			(10, "0xa"),
			(-14, "-0xe"),
			(999, "0x3e7"),
			(-2026, "-0x7ea"),
			(9383766512304, "0x888d411d2b0"),
			(i64::MAX as i128, "0x7fffffffffffffff"),
			(i64::MIN as i128, "-0x8000000000000000"),
			(i128::MAX, "0x7fffffffffffffffffffffffffffffff"),
			(i128::MIN, "-0x80000000000000000000000000000000"),
		] {
			let mut out = IoWriter::new_bytes();

			out.hex_i128(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn dec_f64() {
		for (n, want) in [
			(f64::INFINITY, "inf"),
			(f64::NEG_INFINITY, "-inf"),
			(f64::NAN, "NaN"),
			(0f64, "0"),
			(-0f64, "-0"),
			(-9.439834, "-9.439834"),
			(10.111, "10.111"),
			(-14.439834, "-14.439834"),
			(999.111, "999.111"),
			(-2026.007, "-2026.007"),
			(9383766512304f64, "9383766512304"),
			(i32::MAX as f64, "2147483647"),
			(i32::MIN as f64, "-2147483648"),
			(
				f64::MAX,
				"1797693134862315700000000000000000000000000000000000000000000000\
				00000000000000000000000000000000000000000000000000000000000000000\
				00000000000000000000000000000000000000000000000000000000000000000\
				00000000000000000000000000000000000000000000000000000000000000000\
				00000000000000000000000000000000000000000000000000",
			),
			(
				f64::MIN,
				"-17976931348623157000000000000000000000000000000000000000000000\
				0000000000000000000000000000000000000000000000000000000000000000\
				0000000000000000000000000000000000000000000000000000000000000000\
				0000000000000000000000000000000000000000000000000000000000000000\
				0000000000000000000000000000000000000000000000000000000",
			),
		] {
			let mut out = IoWriter::new_bytes();

			out.dec_f64(&n).unwrap();
			assert_eq!(out.into_string().unwrap(), want);
		}
	}
}
