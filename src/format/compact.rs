//! [Format]ter for compact text output.
//!
//! Outputs one line per log entry:
//1 `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=[value_2, value_3]`
use ntime::Format;
use std::io;

use crate::attributes::{Scalar, StringIndexContainer, Value};
use crate::constant::DEFAULT_LOG_DELIMITER_STRING;
use crate::encoding;
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::LogUpdate;
use crate::write::IoWriter;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Compact`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Compact,
		time_format: Format::DateTime(ntime::Timezone::Local, ntime::Precision::Milliseconds),
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

/// Serializes a [`Scalar`] for [`OutputFormat::Compact`] into a [`write:IoWriter`].
pub fn write_scalar(out: &mut IoWriter, update: &LogUpdate, s: &Scalar) -> io::Result<()> {
	match s {
		Scalar::None => out.str("<none>"),
		Scalar::Bool(b) => out.bool(b),
		Scalar::String(s, escape) => encoding::write_quoted_str(out, s.as_str(), if *escape { &encoding::Mode::Utf8Escaped } else { &encoding::Mode::Utf8 }),
		Scalar::StringSlice(s, escape) => encoding::write_quoted_str(out, s, if *escape { &encoding::Mode::Utf8Escaped } else { &encoding::Mode::Utf8 }),
		Scalar::StringIndex(idx, escape) => encoding::write_quoted_str(out, update.str_by_idx(*idx), if *escape { &encoding::Mode::Utf8Escaped } else { &encoding::Mode::Utf8 }),
		Scalar::Int(i) => out.dec_i64(i),
		Scalar::LongInt(i) => out.hex_i128(i),
		Scalar::Size(s) => out.hex_isize(s),
		Scalar::Uint(i) => out.dec_u64(i),
		Scalar::LongUint(u) => out.hex_u128(u),
		Scalar::Usize(u) => out.hex_usize(u),
		Scalar::Float(f) => out.dec_f64(f),
	}
}

/// Serializes a [`Value`]s for [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write_value(out: &mut IoWriter, update: &LogUpdate, val: &Value) -> io::Result<()> {
	match val {
		Value::Scalar(s) => write_scalar(out, update, s),
		Value::List(ss) => {
			out.u8(b'[')?;
			for i in 0..ss.len() {
				if i != 0 {
					out.str(", ")?;
				}
				write_scalar(out, update, &ss[i])?;
			}
			out.u8(b']')
		}
		Value::Map(keys, ss) => {
			out.u8(b'{')?;
			for i in 0..keys.len() {
				if i != 0 {
					out.str(", ")?;
				}
				write_scalar(out, update, &keys[i])?;
				out.str(": ")?;
				write_scalar(out, update, &ss[i])?;
			}
			out.u8(b'}')
		}
	}
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::Compact`] into a [`io::Write`].
pub fn write(mut out: &mut IoWriter, time_format: &Format, update: &LogUpdate) -> io::Result<()> {
	// build output header: "timestamp [level] message"
	update.when().write(&mut out, time_format)?;
	out.str(" [")?;
	out.str(update.level().as_short_str())?;
	out.str("] ")?;
	out.str(update.message())?;

	// append fields: " key=value"
	for (key, val) in update.attribute_iter() {
		out.space()?;
		out.str(key)?;
		out.u8(b'=')?;
		write_value(out, update, &val)?;
	}

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::attributes::{Map, Scalar, Value};
	use crate::level::Level;
	use ntime::Timestamp;

	#[test]
	fn serialize_scalar() {
		for tc in [
			(Scalar::None, "<none>"),
			(Scalar::from(true), "true"),
			(Scalar::from(""), "\"\""),
			(Scalar::from("abcd 1234"), "\"abcd 1234\""),
			(Scalar::from("quizás\n\"lala\""), "\"quiz\\u{e1}s\\n\\\"lala\\\"\""),
			(Scalar::from(-123), "-123"),
			(Scalar::from(-12345678901234567_i128), "-0x2bdc545d6b4b87"),
			(Scalar::from(89801234567890123_isize), "0x13f09bf3ecf84cb"),
			(Scalar::from(123456), "123456"),
			(Scalar::from(12345678901234567_u128), "0x2bdc545d6b4b87"),
			(Scalar::from(89801234567890123_usize), "0x13f09bf3ecf84cb"),
			(Scalar::from(-1.2345), "-1.2345"),
		] {
			let (s, want): (Scalar, &str) = tc;
			let mut out = IoWriter::new_bytes();

			let when = Timestamp::now();
			let attrs = Map::new();
			let update = LogUpdate::from((&when, &attrs));

			assert!(write_scalar(&mut out, &update, &s).is_ok());
			assert_eq!(out.into_string().unwrap(), String::from(want));
		}
	}

	#[test]
	fn serialize_value() {
		for tc in [
			(Value::from(Scalar::None), "<none>"),
			(Value::from(true), "true"),
			(Value::from(89801234567890123_usize), "0x13f09bf3ecf84cb"),
			(
				Value::from(&[
					Scalar::from(false),
					Scalar::from("abcd 1234"),
					Scalar::from(-123),
					Scalar::from(None::<bool>),
					Scalar::from(89801234567890123_usize),
					Scalar::from(5678901.2345),
				]),
				"[false, \"abcd 1234\", -123, <none>, 0x13f09bf3ecf84cb, 5678901.2345]",
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
			let mut out = IoWriter::new_bytes();

			let when = Timestamp::now();
			let attrs = Map::new();
			let update = LogUpdate::from((&when, &attrs));

			assert!(write_value(&mut out, &update, &v).is_ok());
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn serialize() {
		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123));
		attrs.insert("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert("nothing", Value::from(None::<f32>));
		attrs.insert("a_list", Value::from(&[Scalar::from(349834934_usize), Scalar::from(true)]));
		attrs.insert("a_map", Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee")])));

		let when = Timestamp::try_from((2026, 4, 12, 17, 56, 39, 123, 456)).expect("failed to initialize timestamp");
		let update = LogUpdate::from((&when, Level::Warning, 2, "test compact update ❤️", &attrs));

		let time_format = &ntime::Format::Timestamp(ntime::Precision::Nanoseconds);
		let want = "1776016599123000456 [WRN] test compact update ❤\u{fe0f} an_int=123 a_float=-456.789 some_string=\"hi there!\" nothing=<none> a_list=[0x14da0eb6, true] a_map={\"key #1\": false, \"key #2\": \"weee\"}";

		let mut out = IoWriter::new_bytes();
		assert!(write(&mut out, time_format, &update).is_ok());
		assert_eq!(out.into_string().unwrap(), String::from(want));
	}
}
