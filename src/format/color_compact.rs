//! [Format]ter for colorized compact text output.
//!
//! Outputs one line per log entry:
//! `2026-01-02 15:16:17.890 INF some log message key_1=value_1 key2=[value_2, value_3]`

use ntime::Format;
use std::io;

use crate::attributes::value::Value;
use crate::attributes::{Metadata, MetadataField, MetadataImpl};
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

// Serializes a [`Value`] for [`OutputFormat::ColorCompact`] into a [`io::Write`].
fn write_value<T: io::Write>(out: &mut T, update: &LogUpdate, val: &Value) -> io::Result<()> {
	compact::write_value(out, update, val)
}

// Computes the message color escape string for an [`LogUpdate`]s.
#[inline]
pub fn message_color(update: &LogUpdate) -> Color {
	// update messages above debug are highlighted
	if Level::Debug.includes(&update.level()) {
		return Color::White;
	}
	Color::BrightWhite
}

// Computes the key color escape string given an attribute's [`Metadata`].
#[inline]
pub fn key_color(meta: Metadata) -> Color {
	// non-ephemeral key names are highlighted
	if meta.get(MetadataField::Ephemeral) {
		return Color::Cyan;
	}
	Color::BrightCyan
}

// Computes the value color escape string given an attribute's [`Metadata`].
#[inline]
pub fn val_color(meta: Metadata) -> Color {
	if meta.get(MetadataField::Error) {
		return Color::BrightRed;
	}
	Color::White
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::ColorCompact`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, update: &LogUpdate) -> io::Result<()> {
	out.write(Color::White.to_escape_str().as_bytes())?;
	update.when().write(out, time_format)?;
	write!(
		out,
		" {level_color}{level} {msg_color}{msg}",
		level_color = update.level().color().to_escape_str(),
		level = update.level().as_short_str(),
		msg_color = message_color(update).to_escape_str(),
		msg = update.message(),
	)?;

	// append fields
	for (key, val, meta) in update.attribute_iter() {
		write!(
			out,
			" {key_color}{key}={val_color}",
			key_color = key_color(meta).to_escape_str(),
			// error attributes are highlighted in red
			val_color = val_color(meta).to_escape_str(),
		)?;
		write_value(out, update, &val)?;
	}

	write!(out, "{color_close}", color_close = Color::Default.to_escape_str())?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::attributes::{Map, Scalar, Value};
	use crate::console;
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
					Scalar::None,
					Scalar::from(89801234567890123 as usize),
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

			let mut out = Vec::new();
			let fixed = Map::new();
			let update = LogUpdate::from(&fixed);

			assert!(write_value(&mut out, &update, &v).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), want);
		}
	}

	#[test]
	fn serialize_color() {
		let mut fixed = Map::new();
		fixed.insert("an_int", Value::from(123 as i32));
		fixed.insert_ephemeral("a_float", Value::from(-456.789));
		fixed.insert("some_string", Value::from("hi there!"));
		fixed.insert("nothing", Value::from(None::<u32>));
		fixed.insert_ephemeral("a_set", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));

		let update = LogUpdate::from((
			Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
			Level::Warning,
			1,
			"test compact update",
			&fixed,
		));
		let time_format = &ntime::Format::TimestampNanoseconds;

		for tc in [
			(
				false,
				"1776016599123000456 WRN test compact update an_int=123 a_float=-456.789 some_string=\"hi there!\" nothing=<none> a_set=[0x14da0eb6, true]",
			),
			(
				true,
				"\u{1b}[37m1776016599123000456 \u{1b}[33mWRN \u{1b}[97mtest compact update \u{1b}[96man_int=\u{1b}[37m123 \u{1b}[36ma_float=\u{1b}[37m-456.789 \u{1b}[96msome_string=\u{1b}[37m\"hi there!\" \u{1b}[96mnothing=\u{1b}[37m<none> \u{1b}[36ma_set=\u{1b}[37m[0x14da0eb6, true]\u{1b}[0m",
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
