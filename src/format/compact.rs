/// Formatter for compact text output.
///
/// `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2`
use ntime::Format;
use std::io;

use crate::attributes::Map;
use crate::attributes::value::Value;
use crate::constant::DEFAULT_LOG_DELIMITER_STRING;
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::LogUpdate;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Compact`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Compact,
		time_format: Format::LocalMillisDateTime,
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

/// Serializes a [`Value`] for [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, val: &Value) -> io::Result<()> {
	match &val {
		Value::Bool(b) => write!(out, "{}", b),
		Value::String(s) => write!(out, "\"{}\"", s),
		Value::Int(i) => write!(out, "{}", i),
		Value::LongInt(i) => {
			if *i < 1 {
				write!(out, "-0x{:x}", -i)
			} else {
				write!(out, "0x{:x}", i)
			}
		}
		Value::Size(s) => {
			if *s < 1 {
				write!(out, "-0x{:x}", -s)
			} else {
				write!(out, "0x{:x}", s)
			}
		}
		Value::Uint(i) => write!(out, "{}", i),
		Value::LongUint(u) => write!(out, "0x{:x}", u),
		Value::Usize(u) => write!(out, "0x{:x}", u),
		Value::Float(f) => write!(out, "{}", f),
	}
}

/// Serializes a [`LogUpdate`], + [attributes][`Map`] as [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, update: &LogUpdate, attrs: &Map) -> io::Result<()> {
	// build output header
	update.when.write(out, time_format)?;
	write!(out, " [{level}] {msg}", level = update.level.as_short_str(), msg = update.msg)?;

	// append fields
	for (key, val) in attrs.into_iter() {
		write!(out, " {key}=")?;
		write_value(out, val)?;
	}

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
	fn serialize_write() {
		for tc in [
			(Value::Bool(true), "true"),
			(Value::String("".into()), "\"\""),
			(Value::String("abcd 1234".into()), "\"abcd 1234\""),
			(Value::Int(-123), "-123"),
			(Value::LongInt(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Value::Size(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Uint(123456), "123456"),
			(Value::LongUint(12345678901234567), "0x2bdc545d6b4b87"),
			(Value::Usize(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Float(-1.2345), "-1.2345"),
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
			"test compact update".into(),
		);
		let time_format = &ntime::Format::TimestampNanoseconds;

		let mut attrs = Map::new();
		attrs.insert("an_int", 123.to_value());
		attrs.insert("a_float", (-456.789).to_value());
		attrs.insert("a_usize", (349834934 as usize).to_value());
		attrs.insert("some_string", "hi there!".to_value());

		let want = "1776016599123000456 [WRN] test compact update an_int=123 a_float=-456.789 a_usize=0x14da0eb6 some_string=\"hi there!\"";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
}
