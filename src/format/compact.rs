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
pub fn write_scalar<T: io::Write>(out: &mut T, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::Bool(b) => write!(out, "{}", b),
		Scalar::String(s) => s.write_quoted_escaped(out),
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
pub fn write_value<T: io::Write>(out: &mut T, val: &Value) -> io::Result<()> {
	match val {
		Value::Scalar(s) => write_scalar(out, &s),
		Value::List(ss) => {
			write!(out, "[")?;
			for i in 0..ss.len() {
				if i != 0 {
					write!(out, ", ")?;
				}
				write_scalar(out, &ss[i])?;
			}
			write!(out, "]")
		}
		Value::Map(keys, ss) => {
			write!(out, "{{")?;
			for i in 0..keys.len() {
				if i != 0 {
					write!(out, ", ")?;
				}
				write_scalar(out, &keys[i])?;
				write!(out, ": ")?;
				write_scalar(out, &ss[i])?;
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
		write_value(out, &val)?;
	}

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
			(Scalar::String("quizás\n\"lala\"".into()), "\"quiz\\u{e1}s\\n\\\"lala\\\"\""),
			(Scalar::Int(-123), "-123"),
			(Scalar::LongInt(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Scalar::Size(89801234567890123), "0x13f09bf3ecf84cb"),
			(Scalar::Uint(123456), "123456"),
			(Scalar::LongUint(12345678901234567), "0x2bdc545d6b4b87"),
			(Scalar::Usize(89801234567890123), "0x13f09bf3ecf84cb"),
			(Scalar::Float(-1.2345), "-1.2345"),
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
			((89801234567890123 as usize).to_value(), "0x13f09bf3ecf84cb"),
			(
				[
					Scalar::Bool(false),
					Scalar::String("abcd 1234".into()),
					Scalar::Int(-123),
					Scalar::Size(89801234567890123),
					Scalar::Float(5678901.2345),
				]
				.to_value(),
				"[false, \"abcd 1234\", -123, 0x13f09bf3ecf84cb, 5678901.2345]",
			),
			(
				(
					["key_a".to_scalar(), "key_b".to_scalar(), "key_c".to_scalar()],
					[false.to_scalar(), (-123).to_scalar(), (456.789).to_scalar()],
				)
					.to_value(),
				"{\"key_a\": false, \"key_b\": -123, \"key_c\": 456.789}",
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
			"test compact update".into(),
		);
		let time_format = &ntime::Format::TimestampNanoseconds;

		let mut attrs = Map::new();
		attrs.insert("an_int", (123 as i32).to_value());
		attrs.insert("a_float", (-456.789).to_value());
		attrs.insert("some_string", "hi there!".to_value());
		attrs.insert("a_set", [(349834934 as usize).to_scalar(), true.to_scalar()].to_value());

		let want = "1776016599123000456 [WRN] test compact update an_int=123 a_float=-456.789 some_string=\"hi there!\" a_set=[0x14da0eb6, true]";
		let mut out = Vec::new();
		assert!(write(&mut out, time_format, &update, &attrs).is_ok());
		assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
	}
}
