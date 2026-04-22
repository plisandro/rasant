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
		Scalar::ShortString(ss) => write!(out, "\"{}\"", ss.as_str()),
		Scalar::String(s) => write!(out, "\"{}\"", s),
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
		Value::Set(ss) => {
			write!(out, "[")?;
			for i in 0..ss.len() {
				if i != 0 {
					write!(out, ",")?;
				}
				write_scalar(out, &ss[i])?;
			}
			write!(out, "]")
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
	use crate::attributes::{ToScalar, ToValue};
	use crate::level::Level;
	use ntime::Timestamp;

	#[test]
	fn serialize_scalar() {
		for tc in [
			(Scalar::Bool(true), "true"),
			(Scalar::String("".into()), "\"\""),
			(Scalar::String("abcd 1234".into()), "\"abcd 1234\""),
			(Scalar::Int(-123), "-123"),
			(Scalar::LongInt(-12345678901234567), "-12345678901234567"),
			(Scalar::LongInt(89801234567890123), "89801234567890123"),
			(Scalar::Size(-12345678901234567), "-12345678901234567"),
			(Scalar::Size(89801234567890123), "89801234567890123"),
			(Scalar::Uint(123456), "123456"),
			(Scalar::LongUint(12345678901234567), "12345678901234567"),
			(Scalar::Usize(89801234567890123), "89801234567890123"),
			(Scalar::Float(-1234.56789012345), "-1.23456789012345e3"),
			(Scalar::Float(5678901.2345), "5.6789012345e6"),
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
			(true.to_value(), "true"),
			((89801234567890123 as usize).to_value(), "89801234567890123"),
			(
				[
					false.to_scalar(),
					"abcd 1234".to_scalar(),
					(-123).to_scalar(),
					(89801234567890123 as isize).to_scalar(),
					(5678901.2345).to_scalar(),
				]
				.to_value(),
				"[false,\"abcd 1234\",-123,89801234567890123,5.6789012345e6]",
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
		attrs.insert("an_int", (123 as i32).to_value());
		attrs.insert("a_float", (-456.789).to_value());
		attrs.insert("some_string", "hi there!".to_value());
		attrs.insert("a_set", [(349834934 as usize).to_scalar(), true.to_scalar()].to_value());

		let want =
			"{\"timestamp\":1776016599123000456,\"level\":\"warning\",\"message\":\"test JSON update\",\"an_int\":123,\"a_float\":-4.56789e2,\"some_string\":\"hi there!\",\"a_set\":[349834934,true]}";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, time_key, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
}
