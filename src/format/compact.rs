/// Formatter for compact text output.
///
/// `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2`
use ntime::Format;
use std::io;

use crate::attributes::{Map, Scalar, Value};
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

/// Serializes a [`Scalar`] for [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write_scalar<T: io::Write>(out: &mut T, attrs: &Map, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::Bool(b) => write!(out, "{}", b),
		Scalar::String(s) => s.write_quoted_escaped(out, attrs),
		Scalar::Int(i) => write!(out, "{}", i),
		Scalar::LongInt(i) => {
			if *i < 1 {
				write!(out, "-0x{:x}", -i)
			} else {
				write!(out, "0x{:x}", i)
			}
		}
		Scalar::Size(s) => {
			if *s < 1 {
				write!(out, "-0x{:x}", -s)
			} else {
				write!(out, "0x{:x}", s)
			}
		}
		Scalar::Uint(i) => write!(out, "{}", i),
		Scalar::LongUint(u) => write!(out, "0x{:x}", u),
		Scalar::Usize(u) => write!(out, "0x{:x}", u),
		Scalar::Float(f) => write!(out, "{}", f),
	}
}

/// Serializes a [`Value`]s for [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, attrs: &Map, val: &Value) -> io::Result<()> {
	match val {
		Value::Scalar(s) => write_scalar(out, attrs, &s),
		Value::List(ss) => {
			write!(out, "[")?;
			for i in 0..ss.len() {
				if i != 0 {
					write!(out, ", ")?;
				}
				write_scalar(out, attrs, &ss[i])?;
			}
			write!(out, "]")
		}
		Value::Map(keys, ss) => {
			write!(out, "{{")?;
			for i in 0..keys.len() {
				if i != 0 {
					write!(out, ", ")?;
				}
				write_scalar(out, attrs, &keys[i])?;
				write!(out, ": ")?;
				write_scalar(out, attrs, &ss[i])?;
			}
			write!(out, "}}")
		}
	}
}

/// Serializes a [`LogUpdate`], + [attributes][`Map`] as [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, update: &LogUpdate, attrs: &Map) -> io::Result<()> {
	// build output header
	update.when.write(out, time_format)?;
	write!(out, " [{level}] {msg}", level = update.level.as_short_str(), msg = update.msg)?;

	// append fields
	for (key, val) in attrs.iter() {
		write!(out, " {key}=")?;
		write_value(out, attrs, &val)?;
	}

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
			(Scalar::from(-12345678901234567 as i128), "-0x2bdc545d6b4b87"),
			(Scalar::from(89801234567890123 as isize), "0x13f09bf3ecf84cb"),
			(Scalar::from(123456), "123456"),
			(Scalar::from(12345678901234567 as u128), "0x2bdc545d6b4b87"),
			(Scalar::from(89801234567890123 as usize), "0x13f09bf3ecf84cb"),
			(Scalar::from(-1.2345), "-1.2345"),
		] {
			let (s, want): (Scalar, &str) = tc;

			let mut out = Vec::new();
			let attrs = Map::new();
			assert!(write_scalar(&mut out, &attrs, &s).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
		}
	}

	#[test]
	fn serialize_value() {
		for tc in [
			(Value::from(true), "true"),
			(Value::from(89801234567890123 as usize), "0x13f09bf3ecf84cb"),
			(
				Value::from(&[
					Scalar::from(false),
					Scalar::from("abcd 1234"),
					Scalar::from(-123),
					Scalar::from(89801234567890123 as usize),
					Scalar::from(5678901.2345),
				]),
				"[false, \"abcd 1234\", -123, 0x13f09bf3ecf84cb, 5678901.2345]",
			),
			(
				Value::from((
					&[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_c")],
					&[Scalar::from(false), Scalar::from(-123), Scalar::from(456.789)],
				)),
				"{\"key_a\": false, \"key_b\": -123, \"key_c\": 456.789}",
			),
		] {
			let (v, want): (Value, &str) = tc;

			let mut out = Vec::new();
			let attrs = Map::new();
			assert!(write_value(&mut out, &attrs, &v).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), want);
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
		attrs.insert("an_int", Value::from(123));
		attrs.insert("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));
		attrs.insert("a_map", Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee")])));

		let want = "1776016599123000456 [WRN] test compact update an_int=123 a_float=-456.789 some_string=\"hi there!\" a_list=[0x14da0eb6, true] a_map={\"key #1\": false, \"key #2\": \"weee\"}";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
}
