/// Formatter for colorized compact text output.
///
/// `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=[value_2, value3]`
use ntime::Format;
use std::io;

use crate::attributes::Map;
use crate::attributes::value::Value;
use crate::console::Color;
use crate::constant::{ATTRIBUTE_KEY_ERROR, DEFAULT_LOG_DELIMITER_STRING};
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
pub fn write_value<T: io::Write>(out: &mut T, val: &Value) -> io::Result<()> {
	compact::write_value(out, val)
}

/// Serializes a [`LogUpdate`], + [attributes][`Map`] as [`OutputFormat::ColorCompact`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, time_format: &Format, update: &LogUpdate, attrs: &Map) -> io::Result<()> {
	// update messages above debug are highlighted in white
	let msg_color = if Level::Debug.includes(&update.level) { Color::Default } else { Color::BrightWhite };
	let level_color = update.level.color();

	update.when.write(out, time_format)?;
	write!(
		out,
		" {level_open}{level}{level_close} {msg_open}{msg}{msg_close}",
		level_open = level_color.to_escape_str(),
		level = update.level.as_short_str(),
		level_close = Color::Default.to_escape_str(),
		msg_open = msg_color.to_escape_str(),
		msg = update.msg,
		msg_close = Color::Default.to_escape_str(),
	)?;

	// append fields
	for (key, val) in attrs.iter() {
		write!(
			out,
			" {key_open}{key}{key_close}={vals_open}",
			key_open = Color::Cyan.to_escape_str(),
			key_close = Color::Default.to_escape_str(),
			// error attributes are highlighted in red
			vals_open = if key == ATTRIBUTE_KEY_ERROR { Color::BrightRed.to_escape_str() } else { "" }
		)?;
		write_value(out, &val)?;
		write!(out, "{vals_close}", vals_close = Color::Default.to_escape_str())?;
	}

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use crate::attributes::{ToScalar, ToValue};
	//use crate::attributes::value::ToValue;
	//use crate::level::Level;
	//use ntime::Timestamp;

	#[test]
	fn serialize_value() {
		for tc in [
			(true.to_value(), "true"),
			((89801234567890123 as usize).to_value(), "0x13f09bf3ecf84cb"),
			(
				[
					false.to_scalar(),
					"abcd 1234".to_scalar(),
					(-123).to_scalar(),
					(89801234567890123 as usize).to_scalar(),
					(5678901.2345).to_scalar(),
				]
				.to_value(),
				"[false, \"abcd 1234\", -123, 0x13f09bf3ecf84cb, 5678901.2345]",
			),
		] {
			let (v, want): (Value, &str) = tc;

			let mut out = Vec::new();
			assert!(write_value(&mut out, &v).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), want);
		}
	}

	// TODO: enable tests once color support can be overriden
	/*
	#[test]
	fn serialize_color() {
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

	#[test]
	fn serialize_no_color() {
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
	*/
}
