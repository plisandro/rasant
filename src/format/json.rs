/// Formatter for JSON output.
///
/// `{"timestamp":123456,"level":"info","message":"some log message","key_1":value_1,"key_2":[value_3, value_4]}`
use ntime::Format;
use std::io;

use crate::attributes::Map;
use crate::attributes::{Scalar, Value};
use crate::constant::{ATTRIBUTE_KEY_MESSAGE, DEFAULT_LOG_DELIMITER_STRING};
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::LogUpdate;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Json`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Json,
		time_format: ntime::Format::TimestampMilliseconds,
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

/// Serializes a [`Scalar`] for [`OutputFormat::Json`] into a [`io::Write`].
pub fn write_scalar<T: io::Write>(out: &mut T, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::Bool(b) => write!(out, "{}", b),
		Scalar::String(s) => s.write_quoted_escaped(out),
		Scalar::Int(i) => write!(out, "{}", i),
		Scalar::LongInt(i) => write!(out, "{}", i),
		Scalar::Size(s) => write!(out, "{}", s),
		Scalar::Uint(i) => write!(out, "{}", i),
		Scalar::LongUint(u) => write!(out, "{}", u),
		Scalar::Usize(s) => write!(out, "{}", s),
		Scalar::Float(f) => write!(out, "{0:e}", f),
	}?;

	Ok(())
}

/// Serializes a [`Value`] for [`OutputFormat::Json`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, val: &Value) -> io::Result<()> {
	match val {
		Value::Scalar(s) => write_scalar(out, &s),
		Value::List(ss) => {
			write!(out, "[")?;
			for i in 0..ss.len() {
				if i != 0 {
					write!(out, ",")?;
				}
				write_scalar(out, &ss[i])?;
			}
			write!(out, "]")
		}
		Value::Map(keys, ss) => {
			write!(out, "{{")?;
			for i in 0..keys.len() {
				if i != 0 {
					write!(out, ",")?;
				}
				write_scalar(out, &keys[i])?;
				write!(out, ":")?;
				write_scalar(out, &ss[i])?;
			}
			write!(out, "}}")
		}
	}
}

/// Serializes a [`LogUpdate`], + [attributes][`Map`] as [`OutputFormat::Json`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, time_key: &str, update: &LogUpdate, attrs: &Map) -> io::Result<()> {
	// build output header
	match time_format.as_integer(&update.when) {
		Some(timestamp_int) => write!(
			out,
			"{{\"{time_key}\":{timestamp_int},\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
			level = update.level.as_str(),
			msg_key = ATTRIBUTE_KEY_MESSAGE,
			msg = update.msg,
		)?,
		None => {
			write!(out, "{{\"{time_key}\":\"")?;
			update.when.write(out, time_format)?;
			write!(
				out,
				"\",\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
				level = update.level.as_str(),
				msg_key = ATTRIBUTE_KEY_MESSAGE,
				msg = update.msg,
			)?;
		}
	}

	// append fields
	for (key, val) in attrs.iter() {
		write!(out, ",\"{key}\":")?;
		write_value(out, &val)?;
	}
	write!(out, "}}")?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use crate::attributes::{Scalar, Value};
	use crate::level::Level;
	use ntime::Timestamp;

	#[test]
	fn serialize_scalar() {
		for tc in [
			(Scalar::from(true), "true"),
			(Scalar::from(""), "\"\""),
			(Scalar::from("abcd 1234"), "\"abcd 1234\""),
			(Scalar::from("quizás\n\"lala\""), "\"quiz\\u{e1}s\\n\\\"lala\\\"\""),
			(Scalar::from(-123), "-123"),
			(Scalar::from(-12345678901234567 as i128), "-12345678901234567"),
			(Scalar::from(89801234567890123 as i128), "89801234567890123"),
			(Scalar::from(-12345678901234567 as isize), "-12345678901234567"),
			(Scalar::from(89801234567890123 as isize), "89801234567890123"),
			(Scalar::from(123456), "123456"),
			(Scalar::from(12345678901234567 as u128), "12345678901234567"),
			(Scalar::from(89801234567890123 as usize), "89801234567890123"),
			(Scalar::from(-1234.56789012345), "-1.23456789012345e3"),
			(Scalar::from(5678901.2345), "5.6789012345e6"),
		] {
			let (s, want): (Scalar, &str) = tc;

			let mut out = Vec::new();
			assert!(write_scalar(&mut out, &s).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
		}
	}
	#[test]
	fn serialize_value() {
		for tc in [
			(Value::from(true), "true"),
			(Value::from(89801234567890123 as usize), "89801234567890123"),
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
		] {
			let (v, want): (Value, &str) = tc;

			let mut out = Vec::new();
			assert!(write_value(&mut out, &v).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), want);
		}
	}

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
}
