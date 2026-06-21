//! [Format]ter for colorized compact text output.
//!
//! Outputs one line per log entry:
//! `2026-01-02 15:16:17.890 INF some log message key_1=value_1 key2=[value_2, value_3]`

use ntime::Format;
use std::io;

use crate::attributes::value::Value;
use crate::attributes::{Map, MetadataField, MetadataImpl};
use crate::console::Color;
use crate::constant::DEFAULT_LOG_DELIMITER_STRING;
use crate::format::compact;
use crate::format::{FormatterConfig, OutputFormat};
use crate::level::Level;
use crate::sink::LogUpdate;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::ColorCompact`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::ColorCompact,
		time_format: ntime::Format::LocalMillisDateTime,
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

/// Serializes a [`Value`] for [`OutputFormat::ColorCompact`] into a [`io::Write`].
pub fn write_value<T: io::Write>(out: &mut T, attrs: &Map, val: &Value) -> io::Result<()> {
	compact::write_value(out, attrs, val)
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::ColorCompact`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, update: &LogUpdate) -> io::Result<()> {
	// update messages above debug are highlighted in white
	let msg_color = if Level::Debug.includes(&update.level()) { Color::Default } else { Color::BrightWhite };
	let level_color = update.level().color();

	update.when().write(out, time_format)?;
	write!(
		out,
		" {level_open}{level}{level_close} {msg_open}{msg}{msg_close}",
		level_open = level_color.to_escape_str(),
		level = update.level().as_short_str(),
		level_close = Color::Default.to_escape_str(),
		msg_open = msg_color.to_escape_str(),
		msg = update.message(),
		msg_close = Color::Default.to_escape_str(),
	)?;

	// append fields
	for (key, val, meta) in update.attributes().iter() {
		write!(
			out,
			" {key_open}{key}{key_close}={vals_open}",
			// non-ephemeral key names are highlighted
			key_open = (if meta.get(MetadataField::Ephemeral) { Color::Cyan } else { Color::BrightCyan }).to_escape_str(),
			key_close = Color::Default.to_escape_str(),
			// error attributes are highlighted in red
			vals_open = if meta.get(MetadataField::Error) { Color::BrightRed.to_escape_str() } else { "" }
		)?;
		write_value(out, update.attributes(), &val)?;
		write!(out, "{vals_close}", vals_close = Color::Default.to_escape_str())?;
	}

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::attributes::{Scalar, Value};
	use crate::console;
	use crate::sink::PartialLogUpdate;
	use ntime::Timestamp;

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
	fn serialize_color() {
		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123 as i32));
		attrs.insert_ephemeral("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert_ephemeral("a_set", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));

		let pupdate = PartialLogUpdate::new(
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
			Level::Warning,
			1,
			"test compact update".into(),
		);
		let update = LogUpdate::from((&pupdate, &attrs));
		let time_format = &ntime::Format::TimestampNanoseconds;

		for tc in [
			(
				false,
				"1776016599123000456 WRN test compact update an_int=123 a_float=-456.789 some_string=\"hi there!\" a_set=[0x14da0eb6, true]",
			),
			(
				true,
				"1776016599123000456 \u{1b}[33mWRN\u{1b}[0m \u{1b}[97mtest compact update\u{1b}[0m \u{1b}[96man_int\u{1b}[0m=123\u{1b}[0m \u{1b}[36ma_float\u{1b}[0m=-456.789\u{1b}[0m \u{1b}[96msome_string\u{1b}[0m=\"hi there!\"\u{1b}[0m \u{1b}[36ma_set\u{1b}[0m=[0x14da0eb6, true]\u{1b}[0m",
			),
		] {
			let (enable, want) = tc;

			let mut out = Vec::new();

			console::colorterm_force(enable);
			assert!(write(&mut out, time_format, &update).is_ok());
			console::colorterm_unforce();
			assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
		}
	}
}
