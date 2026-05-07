/// Formatter for [CBOR](https://cbor.io/) (RFC 8939) binary output.
use ntime::Format;
use ntime::Timestamp;
use std::io;

use crate::attributes::Map;
use crate::attributes::{Scalar, Value};
use crate::constant::{ATTRIBUTE_KEY_MESSAGE, DEFAULT_LOG_DELIMITER_STRING};
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::LogUpdate;

const MAX_8_BITS: u64 = (u8::MAX as u64) + 1;
const MAX_16_BITS: u64 = (u16::MAX as u64) + 1;
const MAX_32_BITS: u64 = (u32::MAX as u64) + 1;
const MAX_64_BITS: u128 = (u64::MAX as u128) + 1;

// See https://cbor.io/ and https://www.rfc-editor.org/rfc/rfc8949.html#jumptable for details.

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Cbor`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Cbor,
		time_format: ntime::Format::TimestampMilliseconds,
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
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

// Serializes a string into a CBOR stream.
fn write_string<T: io::Write>(out: &mut T, s: &str) -> io::Result<()> {
	write_u64_with_major(out, s.len() as u64, 3 << 5)?;
	_ = out.write(s.as_bytes())?;

	Ok(())
}

// Serializes a double precision float into a CBOR stream.
fn write_float<T: io::Write>(out: &mut T, f: f64) -> io::Result<()> {
	// can we downcast to single precision?
	let sf = f as f32;
	println!("{} => {} => {}", f, sf, (sf as f64));
	match (sf as f64) == f {
		true => {
			println!("f32");
			// type 7, single precision float
			_ = out.write(&[((7 << 5) + 24) as u8])?;
			_ = out.write(sf.to_be_bytes().as_slice());
		}
		false => {
			println!("f64");
			// type 7, double precision float
			_ = out.write(&[((7 << 5) + 25) as u8])?;
			_ = out.write(f.to_be_bytes().as_slice());
		}
	};

	Ok(())
}

// Serializes a timestamp into a CBOR stream; turns out there're dedicated types for second timestamps and RFC3339,
fn write_timestamp<T: io::Write>(out: &mut T, t: &Timestamp, f: &Format) -> io::Result<()> {
	if *f == Format::UtcRFC3339 || *f == Format::LocalRFC3339 {
		// TODO: delete temporary string buffer
		let s = t.as_string(f);
		// major type 6, tag 0 (date/time string)
		_ = out.write(&[((6 << 5) + 0) as u8])?;
		return write_string(out, s.as_str());
	}

	if let Some(i) = t.as_integer(f) {
		if *f == Format::TimestampSeconds {
			// major type 6, tag 1 (epoch timestamp in seconds)
			_ = out.write(&[((6 << 5) + 1) as u8])?;
		}
		return write_u128(out, i);
	}

	// write as string by default.
	// TODO: reuse string buffer
	let s = t.as_string(f);
	write_string(out, s.as_str())
}

/// Serializes a [`Scalar`] for [`OutputFormat::Cbor`] into a [`io::Write`].
pub fn write_scalar<T: io::Write>(out: &mut T, attrs: &Map, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::Bool(b) => write_bool(out, *b),
		Scalar::String(s) => write_string(out, s.as_str(attrs)),
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

/// Serializes a [`Value`] for [`OutputFormat::Json`] into a [`io::Write`].
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

/// Serializes a [`LogUpdate`], + [attributes][`Map`] as [`OutputFormat::Cbor`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, time_key: &str, update: &LogUpdate, attrs: &Map) -> io::Result<()> {
	// write output as a map (major type 5)
	write_u64_with_major(out, (attrs.len() + 2) as u64, 5 << 5)?;

	// time / timestamp
	_ = write_string(out, time_key)?;
	write_timestamp(out, &update.when, time_format)?;

	// message
	_ = write_string(out, ATTRIBUTE_KEY_MESSAGE)?;
	_ = write_string(out, &update.msg)?;

	// attributess
	for (key, val) in attrs.iter() {
		_ = write_string(out, key)?;
		write_value(out, attrs, &val)?;
	}

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

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
			//(Scalar::from(123.456), [0x40, 0x5E, 0xDD, 0x2F, 0x1A, 0x9F, 0xBE, 0x77].as_slice()),
			(Scalar::from(123.45600128173828), [0xfa, 0x42, 0xF6, 0xE9, 0x79].as_slice()),
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
			// ""2026-05-07 18:04:34 +0200"
			(
				Format::LocalDateTime,
				[
					0x78, 0x19, 0x32, 0x30, 0x32, 0x36, 0x2d, 0x30, 0x35, 0x2d, 0x30, 0x37, 0x20, 0x31, 0x38, 0x3a, 0x30, 0x34, 0x3a, 0x33, 0x34, 0x20, 0x2b, 0x30, 0x32, 0x30, 0x30,
				]
				.as_slice(),
			),
		] {
			let (f, want): (Format, &[u8]) = tc;

			let mut out = Vec::new();
			assert!(write_timestamp(&mut out, &ts, &f).is_ok());
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
			/*
			(
				Value::from(&[
					Scalar::from(false),
					Scalar::from("abcd 1234"),
					Scalar::from(123),
					Scalar::from(-89801234567890123 as isize),
					Scalar::from(5678901.2345),
				]),
				"[false,\"abcd 1234\",123,-89801234567890123,5.6789012345e6]",
			),
			(
				Value::from((
					&[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_c")],
					&[Scalar::from(false), Scalar::from(-123), Scalar::from(456.789)],
				)),
				"{\"key_a\":false,\"key_b\":-123,\"key_c\":4.56789e2}",
			),
			 */
		] {
			let (v, want): (Value, &[u8]) = tc;

			let mut out = Vec::new();
			let attrs = Map::new();
			assert!(write_value(&mut out, &attrs, &v).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}

	/*
	#[test]
	fn serialize() {
		let update = LogUpdate::new(
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
			Level::Warning,
			"test JSON update".into(),
		);
		let time_key: &str = "timestamp";
		let time_format = &ntime::Format::TimestampNanoseconds;

		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123 as i32));
		attrs.insert("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));
		attrs.insert("a_map", Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee")])));

		let want = "{\"timestamp\":1776016599123000456,\"level\":\"warning\",\"message\":\"test JSON update\",\"an_int\":123,\"a_float\":-4.56789e2,\"some_string\":\"hi there!\",\"a_list\":[349834934,true],\"a_map\":{\"key #1\":false,\"key #2\":\"weee\"}}";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, time_key, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
	*/
}
