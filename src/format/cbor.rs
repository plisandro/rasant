//! [Format]ter for [CBOR](https://cbor.io/) (RFC 8949) binary output.
//! See <https://cbor.io/> and <https://www.rfc-editor.org/rfc/rfc8949.html#jumptable> for details.
//!
//! Outputs one CBOR-formatted map per log entry.

use ntime::Format;
use ntime::Timestamp;
use std::io;

use crate::attributes::Map;
use crate::attributes::{Scalar, Value};
use crate::constant::ATTRIBUTE_KEY_MESSAGE;
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::LogUpdate;

const MAX_8_BITS: u64 = (u8::MAX as u64) + 1;
const MAX_16_BITS: u64 = (u16::MAX as u64) + 1;
const MAX_32_BITS: u64 = (u32::MAX as u64) + 1;
const MAX_64_BITS: u128 = (u64::MAX as u128) + 1;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Cbor`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Cbor,
		time_format: ntime::Format::TimestampMilliseconds,
		delimiter: [].into(), // no separator
	}
}

// Serializes a boolean into a CBOR stream.
fn write_bool<T: io::Write>(out: &mut T, b: bool) -> io::Result<()> {
	match b {
		// tiny field type 7 (special)
		false => _ = out.write(&[((7 << 5) + 20) as u8])?,
		true => _ = out.write(&[((7 << 5) + 21) as u8])?,
	};

	Ok(())
}

// Serializes a 64-bit unsigned integer into a CBOR stream, for a given major type.
fn write_u64_with_major<T: io::Write>(out: &mut T, u: u64, major_mask: u8) -> io::Result<()> {
	match u {
		// tiny field
		0..24 => _ = out.write(&[major_mask | (u as u8)])?,
		// short field
		24..MAX_8_BITS => _ = out.write(&[major_mask | 24 as u8, u as u8])?,
		MAX_8_BITS..MAX_16_BITS => {
			_ = out.write(&[major_mask | 25 as u8])?;
			_ = out.write((u as u16).to_be_bytes().as_slice())?;
		}
		// long fields
		MAX_16_BITS..MAX_32_BITS => {
			_ = out.write(&[major_mask | 26 as u8])?;
			_ = out.write((u as u32).to_be_bytes().as_slice())?;
		}
		MAX_32_BITS.. => {
			_ = out.write(&[major_mask | 27 as u8])?;
			_ = out.write((u as u64).to_be_bytes().as_slice())?;
		}
	};

	Ok(())
}

// Serializes a 128-bit unsigned integer into a CBOR stream, for a given major type.
fn write_u128_with_major<T: io::Write>(out: &mut T, u: u128, major_mask: u8) -> io::Result<()> {
	match u {
		0..MAX_64_BITS => write_u64_with_major(out, u as u64, major_mask)?,
		// bignum (tag 2 extension)
		MAX_64_BITS.. => {
			_ = out.write(&[((6 << 5) + 2) as u8, ((2 << 5) + 8) as u8])?;
			_ = out.write(u.to_be_bytes().as_slice())?;
		}
	};

	Ok(())
}

// Serializes a 64-bit unsigned integer into a CBOR stream.
fn write_u64<T: io::Write>(out: &mut T, u: u64) -> io::Result<()> {
	write_u64_with_major(out, u, 0)
}

// Serializes a 128-bit unsigned integer into a CBOR stream.
fn write_u128<T: io::Write>(out: &mut T, u: u128) -> io::Result<()> {
	write_u128_with_major(out, u, 0)
}

// Serializes a 64-bit signed integer into a CBOR stream.
fn write_i64<T: io::Write>(out: &mut T, i: i64) -> io::Result<()> {
	if i >= 0 {
		return write_u64_with_major(out, i as u64, 0);
	}

	let n = ((-1) - i) as u64;
	write_u64_with_major(out, n, 1 << 5)
}

// Serializes a 128-bit signed integer into a CBOR stream.
fn write_i128<T: io::Write>(out: &mut T, i: i128) -> io::Result<()> {
	if i >= 0 {
		return write_u128_with_major(out, i as u128, 0);
	}

	let n = ((-1) - i) as u128;
	write_u128_with_major(out, n, 1 << 5)
}

// Serializes a string (as bytes) into a CBOR stream.
fn write_string_bytes<T: io::Write>(out: &mut T, s: &[u8]) -> io::Result<()> {
	write_u64_with_major(out, s.len() as u64, 3 << 5)?;
	_ = out.write(s)?;

	Ok(())
}

// Serializes a string into a CBOR stream.
fn write_string<T: io::Write>(out: &mut T, s: &str) -> io::Result<()> {
	write_string_bytes(out, s.as_bytes())
}

// Serializes a double precision float into a CBOR stream.
fn write_float<T: io::Write>(out: &mut T, f: f64) -> io::Result<()> {
	// TODO: is this the best possible way to downcast?
	let sf = f as f32;
	match (sf as f64) == f {
		true => {
			// type 7, single precision float
			_ = out.write(&[((7 << 5) + 26) as u8])?;
			_ = out.write(sf.to_be_bytes().as_slice());
		}
		false => {
			// type 7, double precision float
			_ = out.write(&[((7 << 5) + 27) as u8])?;
			_ = out.write(f.to_be_bytes().as_slice());
		}
	};

	Ok(())
}

// Serializes a timestamp into a CBOR stream; turns out there're dedicated types for second timestamps and RFC3339.
fn write_timestamp<T: io::Write>(out: &mut T, buf: &mut Vec<u8>, t: &Timestamp, f: &Format) -> io::Result<()> {
	if f.is_rfc_3339() {
		// major type 6, tag 0 (date/time string)
		_ = out.write(&[((6 << 5) + 0) as u8])?;
		buf.clear();
		_ = f.write(buf, t);
		return write_string_bytes(out, buf);
	}

	if let Some(i) = t.as_integer(f) {
		if *f == Format::TimestampSeconds {
			// major type 6, tag 1 (epoch timestamp in seconds)
			_ = out.write(&[((6 << 5) + 1) as u8])?;
		}
		return write_u128(out, i);
	}

	// write as string by default.
	buf.clear();
	_ = f.write(buf, t);
	write_string_bytes(out, buf)
}

/// Serializes a [`Scalar`] for [`OutputFormat::Cbor`] into a [`io::Write`].
pub fn write_scalar<T: io::Write>(out: &mut T, attrs: &Map, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::Bool(b) => write_bool(out, *b),
		Scalar::String(s, _) => write_string(out, s.as_str()),
		Scalar::StringSlice(s, _) => write_string(out, s),
		Scalar::StringIndex(i, _) => write_string(out, attrs.str_by_idx(*i)),
		Scalar::Int(i) => write_i64(out, *i),
		Scalar::LongInt(i) => write_i128(out, *i),
		Scalar::Size(s) => write_i128(out, *s as i128),
		Scalar::Uint(u) => write_u64(out, *u),
		Scalar::LongUint(u) => write_u128(out, *u),
		Scalar::Usize(s) => write_u128(out, *s as u128),
		Scalar::Float(f) => write_float(out, *f),
	}?;

	Ok(())
}

/// Serializes a [`Value`] for [`OutputFormat::Cbor`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, attrs: &Map, val: &Value) -> io::Result<()> {
	match val {
		Value::Scalar(s) => write_scalar(out, attrs, &s)?,
		Value::List(ss) => {
			// major type 4 (array)
			write_u64_with_major(out, ss.len() as u64, 4 << 5)?;
			for i in 0..ss.len() {
				write_scalar(out, attrs, &ss[i])?;
			}
		}
		Value::Map(keys, ss) => {
			// major type 5 (map)
			write_u64_with_major(out, ss.len() as u64, 5 << 5)?;
			for i in 0..keys.len() {
				write_scalar(out, attrs, &keys[i])?;
				write_scalar(out, attrs, &ss[i])?;
			}
		}
	}

	Ok(())
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::Cbor`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, work_buffer: &mut Vec<u8>, time_format: &Format, time_key: &str, update: &LogUpdate) -> io::Result<()> {
	// write output as a map (major type 5)
	write_u64_with_major(out, (update.attributes().len() + 2) as u64, 5 << 5)?;

	// time / timestamp
	_ = write_string(out, time_key)?;
	write_timestamp(out, work_buffer, &update.when(), time_format)?;

	// message
	_ = write_string(out, ATTRIBUTE_KEY_MESSAGE)?;
	_ = write_string(out, &update.message())?;

	// attributess
	for (key, val) in update.attributes().iter() {
		_ = write_string(out, key)?;
		write_value(out, update.attributes(), &val)?;
	}

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::Level;
	use crate::sink::PartialLogUpdate;

	#[test]
	fn serialize_scalar() {
		for tc in [
			(Scalar::from(false), [0xf4].as_slice()),
			(Scalar::from(true), [0xf5].as_slice()),
			// tiny integer
			(Scalar::from(0x00), [0x00].as_slice()),
			(Scalar::from(0x01), [0x01].as_slice()),
			(Scalar::from(0x17), [0x17].as_slice()),
			// single byte integer
			(Scalar::from(0x18), [0x18, 0x18].as_slice()),
			(Scalar::from(0xcd), [0x18, 0xcd].as_slice()),
			(Scalar::from(0xff), [0x18, 0xff].as_slice()),
			// 2-bytes integer
			(Scalar::from(0x0100), [0x19, 0x01, 0x00].as_slice()),
			(Scalar::from(0xabcd), [0x19, 0xab, 0xcd].as_slice()),
			(Scalar::from(0xffff), [0x19, 0xff, 0xff].as_slice()),
			// 4-bytes integer
			(Scalar::from(0x00010000), [0x1a, 0x00, 0x01, 0x00, 0x00].as_slice()),
			(Scalar::from(0x1234abcd), [0x1a, 0x12, 0x34, 0xab, 0xcd].as_slice()),
			(Scalar::from(0xffffffff as u64), [0x1a, 0xff, 0xff, 0xff, 0xff].as_slice()),
			// 8-bytes long integer
			(Scalar::from(0x0000000100000000 as u128), [0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00].as_slice()),
			(Scalar::from(0x1234567890abcdef as u128), [0x1b, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef].as_slice()),
			// negatives
			(Scalar::from(-0x01), [0x20].as_slice()),
			(Scalar::from(-0x18), [0x37].as_slice()),
			(Scalar::from(-0x19), [0x38, 0x18].as_slice()),
			(Scalar::from(-0xcd), [0x38, 0xcc].as_slice()),
			(Scalar::from(-0x0100), [0x38, 0xff].as_slice()),
			(Scalar::from(-0x0101), [0x39, 0x01, 0x00].as_slice()),
			(Scalar::from(-0xabcd), [0x39, 0xab, 0xcc].as_slice()),
			(Scalar::from(-0xffff), [0x39, 0xff, 0xfe].as_slice()),
			(Scalar::from(-0x00010000), [0x39, 0xff, 0xff].as_slice()),
			(Scalar::from(-0x00010001), [0x3a, 0x00, 0x01, 0x00, 0x00].as_slice()),
			(Scalar::from(-0x1234abcd), [0x3a, 0x12, 0x34, 0xab, 0xcc].as_slice()),
			(Scalar::from(-0xffffff as i64), [0x3a, 0x00, 0xff, 0xff, 0xfe].as_slice()),
			(Scalar::from(-0x000000100000000 as i128), [0x3a, 0xff, 0xff, 0xff, 0xff].as_slice()),
			(Scalar::from(-0x234567890abcdef as i128), [0x3b, 0x02, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xee].as_slice()),
			// strings
			(Scalar::from(""), [0x60].as_slice()),
			(Scalar::from("abcd 1234"), [0x69, 0x61, 0x62, 0x63, 0x64, 0x20, 0x31, 0x32, 0x33, 0x34].as_slice()),
			(
				Scalar::from("quizás\n\"lala\""),
				[0x6E, 0x71, 0x75, 0x69, 0x7A, 0xC3, 0xA1, 0x73, 0x0A, 0x22, 0x6C, 0x61, 0x6C, 0x61, 0x22].as_slice(),
			),
			// floats
			(Scalar::from(2.666015625), [0xfa, 0x40, 0x2a, 0xa0, 0x00].as_slice()),
			(Scalar::from(3.1415926535897), [0xfb, 0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2c, 0x46].as_slice()),
		] {
			let (s, want): (Scalar, &[u8]) = tc;

			let mut out = Vec::new();
			let attrs = Map::new();
			assert!(write_scalar(&mut out, &attrs, &s).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}

	#[test]
	fn serialize_timestamp() {
		let ts = Timestamp::from_utc_date(2026, 05, 07, 16, 04, 34, 123, 456).unwrap();
		for tc in [
			// 1778169874; second timestamps get CBOR tag 1
			(Format::TimestampSeconds, [0xc1, 0x1a, 0x69, 0xfc, 0xb8, 0x12].as_slice()),
			// 1778169874123000456
			(Format::TimestampNanoseconds, [0x1b, 0x18, 0xad, 0x54, 0x14, 0x51, 0x67, 0x0a, 0x88].as_slice()),
			// "2026-05-07T16:04:34Z"; RFC 3339 strings get CBOR tag 0
			(
				Format::UtcRFC3339,
				[
					0xc0, 0x74, 0x32, 0x30, 0x32, 0x36, 0x2d, 0x30, 0x35, 0x2d, 0x30, 0x37, 0x54, 0x31, 0x36, 0x3a, 0x30, 0x34, 0x3a, 0x33, 0x34, 0x5a,
				]
				.as_slice(),
			),
			// "2026-05-07 16:04:34.123000456"
			(
				Format::UtcNanosDateTime,
				[
					0x78, 0x1d, 0x32, 0x30, 0x32, 0x36, 0x2d, 0x30, 0x35, 0x2d, 0x30, 0x37, 0x20, 0x31, 0x36, 0x3a, 0x30, 0x34, 0x3a, 0x33, 0x34, 0x2e, 0x31, 0x32, 0x33, 0x30, 0x30, 0x30, 0x34, 0x35,
					0x36,
				]
				.as_slice(),
			),
		] {
			let (f, want): (Format, &[u8]) = tc;

			let mut buffer = Vec::new();
			let mut out = Vec::new();
			assert!(write_timestamp(&mut out, &mut buffer, &ts, &f).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}

	#[test]
	fn serialize_value() {
		for tc in [
			(Value::from("lalala"), [0x66, 0x6c, 0x61, 0x6c, 0x61, 0x6c, 0x61].as_slice()),
			(Value::from(-1234), [0x39, 0x04, 0xd1].as_slice()),
			(Value::from(true), [0xf5].as_slice()),
			(Value::from(89801234567890123 as usize), [0x1b, 0x01, 0x3f, 0x09, 0xbf, 0x3e, 0xcf, 0x84, 0xcb].as_slice()),
			(
				Value::from(&[
					Scalar::from(false),
					Scalar::from("abcd 1234"),
					Scalar::from(123),
					Scalar::from(-89801234567890123 as isize),
					Scalar::from(5678901.2345),
				]),
				[
					0x85, 0xf4, 0x69, 0x61, 0x62, 0x63, 0x64, 0x20, 0x31, 0x32, 0x33, 0x34, 0x18, 0x7b, 0x3b, 0x01, 0x3f, 0x09, 0xbf, 0x3e, 0xcf, 0x84, 0xca, 0xfb, 0x41, 0x55, 0xa9, 0xcd, 0x4f, 0x02,
					0x0c, 0x4a,
				]
				.as_slice(),
			),
			(
				Value::from((
					&[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_c")],
					&[Scalar::from(false), Scalar::from(-123), Scalar::from(456.789)],
				)),
				[
					0xa3, 0x65, 0x6b, 0x65, 0x79, 0x5f, 0x61, 0xf4, 0x65, 0x6b, 0x65, 0x79, 0x5f, 0x62, 0x38, 0x7a, 0x65, 0x6b, 0x65, 0x79, 0x5f, 0x63, 0xfb, 0x40, 0x7c, 0x8c, 0x9f, 0xbe, 0x76, 0xc8,
					0xb4,
				]
				.as_slice(),
			),
		] {
			let (v, want): (Value, &[u8]) = tc;

			let mut out = Vec::new();
			let attrs = Map::new();
			assert!(write_value(&mut out, &attrs, &v).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}

	#[test]
	fn serialize_single() {
		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123 as i32));
		attrs.insert("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));
		attrs.insert("a_map", Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee")])));

		let pupdate = PartialLogUpdate::new(
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
			Level::Warning,
			"test CBOR update".into(),
		);

		let update = LogUpdate::from((&pupdate, &attrs));

		let time_key: &str = "timestamp";
		let time_format = &ntime::Format::TimestampNanoseconds;

		let want = [
			0xa7, 0x69, 0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70, 0x1b, 0x18, 0xa5, 0xad, 0xaf, 0xe9, 0xec, 0x7c, 0x88, 0x67, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x70, 0x74, 0x65,
			0x73, 0x74, 0x20, 0x43, 0x42, 0x4f, 0x52, 0x20, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x66, 0x61, 0x6e, 0x5f, 0x69, 0x6e, 0x74, 0x18, 0x7b, 0x67, 0x61, 0x5f, 0x66, 0x6c, 0x6f, 0x61, 0x74,
			0xfb, 0xc0, 0x7c, 0x8c, 0x9f, 0xbe, 0x76, 0xc8, 0xb4, 0x6b, 0x73, 0x6f, 0x6d, 0x65, 0x5f, 0x73, 0x74, 0x72, 0x69, 0x6e, 0x67, 0x69, 0x68, 0x69, 0x20, 0x74, 0x68, 0x65, 0x72, 0x65, 0x21,
			0x66, 0x61, 0x5f, 0x6c, 0x69, 0x73, 0x74, 0x82, 0x1a, 0x14, 0xda, 0x0e, 0xb6, 0xf5, 0x65, 0x61, 0x5f, 0x6d, 0x61, 0x70, 0xa2, 0x66, 0x6b, 0x65, 0x79, 0x20, 0x23, 0x31, 0xf4, 0x66, 0x6b,
			0x65, 0x79, 0x20, 0x23, 0x32, 0x64, 0x77, 0x65, 0x65, 0x65,
		];

		let mut buffer = Vec::new();
		let mut out = Vec::new();
		assert!(write(&mut out, &mut buffer, time_format, time_key, &update).is_ok());
		assert_eq!(out, want);
	}

	#[test]
	fn serialize_multi() {
		let time_key: &str = "timestamp";
		let time_format = &ntime::Format::TimestampNanoseconds;
		let mut attrs = Map::new();
		let mut buffer = Vec::new();
		let mut out = Vec::new();

		// update #1
		let pupdate = PartialLogUpdate::new(
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
			Level::Warning,
			"test CBOR update #1".into(),
		);
		attrs.clear();
		attrs.insert("an_int", Value::from(123 as i32));
		attrs.insert("a_float", Value::from(-456.789));

		let update = LogUpdate::from((&pupdate, &attrs));
		assert!(write(&mut out, &mut buffer, time_format, time_key, &update).is_ok());

		// update #2
		let pupdate = PartialLogUpdate::new(
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 789, 012).expect("failed to initialize timestamp"),
			Level::Info,
			"test CBOR update #2".into(),
		);
		attrs.clear();
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));

		let update = LogUpdate::from((&pupdate, &attrs));
		assert!(write(&mut out, &mut buffer, time_format, time_key, &update).is_ok());

		let want = [
			0xa4, 0x69, 0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70, 0x1b, 0x18, 0xa5, 0xad, 0xaf, 0xe9, 0xec, 0x7c, 0x88, 0x67, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x73, 0x74, 0x65,
			0x73, 0x74, 0x20, 0x43, 0x42, 0x4f, 0x52, 0x20, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x20, 0x23, 0x31, 0x66, 0x61, 0x6e, 0x5f, 0x69, 0x6e, 0x74, 0x18, 0x7b, 0x67, 0x61, 0x5f, 0x66, 0x6c,
			0x6f, 0x61, 0x74, 0xfb, 0xc0, 0x7c, 0x8c, 0x9f, 0xbe, 0x76, 0xc8, 0xb4, 0xa4, 0x69, 0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70, 0x1b, 0x18, 0xa5, 0xad, 0xb0, 0x11, 0x9e, 0xd5,
			0x4c, 0x67, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x73, 0x74, 0x65, 0x73, 0x74, 0x20, 0x43, 0x42, 0x4f, 0x52, 0x20, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x20, 0x23, 0x32, 0x6b, 0x73,
			0x6f, 0x6d, 0x65, 0x5f, 0x73, 0x74, 0x72, 0x69, 0x6e, 0x67, 0x69, 0x68, 0x69, 0x20, 0x74, 0x68, 0x65, 0x72, 0x65, 0x21, 0x66, 0x61, 0x5f, 0x6c, 0x69, 0x73, 0x74, 0x82, 0x1a, 0x14, 0xda,
			0x0e, 0xb6, 0xf5,
		];
		assert_eq!(out, want);
	}
}
