/// Formatter for JSON output.
///
/// `{"timestamp":123456,"level":"info","message":"some log message","key_1":"=value_1","key_2":"=value_2"}`
use ntime::Format;
use std::io;

use crate::attributes::Map;
use crate::attributes::value::Value;
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

/// Serializes a [`Value`] for [`OutputFormat::Json`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, val: &Value) -> io::Result<()> {
	match val {
		Value::Bool(b) => write!(out, "{}", b),
		Value::String(s) => write!(out, "\"{}\"", s),
		Value::Int(i) => write!(out, "{}", i),
		Value::LongInt(i) => write!(out, "{}", i),
		Value::Size(s) => write!(out, "{}", s),
		Value::Uint(i) => write!(out, "{}", i),
		Value::LongUint(u) => write!(out, "{}", u),
		Value::Usize(s) => write!(out, "{}", s),
		Value::Float(f) => write!(out, "{0:e}", f),
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
	for (key, val) in attrs.into_iter() {
		write!(out, ",\"{key}\":")?;
		write_value(out, val)?;
	}
	write!(out, "}}")?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use crate::attributes::value::ToValue;
	use crate::level::Level;
	use ntime::Timestamp;

	#[test]
	fn serialize_value() {
		for tc in [
			(Value::Bool(true), "true"),
			(Value::String("".into()), "\"\""),
			(Value::String("abcd 1234".into()), "\"abcd 1234\""),
			(Value::Int(-123), "-123"),
			(Value::LongInt(-12345678901234567), "-12345678901234567"),
			(Value::LongInt(89801234567890123), "89801234567890123"),
			(Value::Size(-12345678901234567), "-12345678901234567"),
			(Value::Size(89801234567890123), "89801234567890123"),
			(Value::Uint(123456), "123456"),
			(Value::LongUint(12345678901234567), "12345678901234567"),
			(Value::Usize(89801234567890123), "89801234567890123"),
			(Value::Float(-1234.56789012345), "-1.23456789012345e3"),
			(Value::Float(5678901.2345), "5.6789012345e6"),
		] {
			let (v, want): (Value, &str) = tc;

			let mut out = Vec::new();
			assert!(write_value(&mut out, &v).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
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
		attrs.insert("an_int", 123.to_value());
		attrs.insert("a_float", (-456.789).to_value());
		attrs.insert("a_usize", (349834934 as usize).to_value());
		attrs.insert("some_string", "hi there!".to_value());

		let want =
			"{\"timestamp\":1776016599123000456,\"level\":\"warning\",\"message\":\"test JSON update\",\"an_int\":123,\"a_float\":-4.56789e2,\"a_usize\":349834934,\"some_string\":\"hi there!\"}";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, time_key, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
}
