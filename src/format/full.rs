//! [Format]ter for full text output.
//!
//! Outputs multiple lines (fixed attributes, ephemeral attributes, message)
//! per hierarchical log entry.
//!
//! ```text
//! 2026-01-02 15:16:17.890 [WARNING] fixed_key_1=value_1
//!                                   ephemeral_key_2=[value_2, value_3]
//!                                   some log message
//! ```

use ntime::Format;
use std::io;
use std::io::Write;

use crate::attributes::value::Value;
use crate::attributes::{Map, MetadataField, MetadataImpl};
use crate::constant::{DEFAULT_LOG_DELIMITER_STRING, FORMAT_FULL_DEPTH_ELLIPSIS, FORMAT_FULL_DEPTH_SEPARATOR, FORMAT_FULL_MAX_DEPTH};
use crate::format::compact;
use crate::format::{FormatterConfig, OutputFormat};
use crate::level::LEVEL_LONG_NAME_MAX_LENGTH;
use crate::sink::{LogDepth, LogUpdate};

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Full`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Full,
		time_format: ntime::Format::LocalMillisDateTime,
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

// Serializes a [`Value`] for [`OutputFormat::ColorFull`] into a [`io::Write`].
fn write_value<T: io::Write>(out: &mut T, attrs: &Map, val: &Value) -> io::Result<()> {
	compact::write_value(out, attrs, val)
}

// Write a spacer based on the [`LogDepth`] for a [`LogUpdate`].
fn write_depth_spacer<T: io::Write>(out: &mut T, depth: LogDepth) -> io::Result<()> {
	if depth <= FORMAT_FULL_MAX_DEPTH {
		for _ in 0..depth {
			write!(out, "{}", FORMAT_FULL_DEPTH_SEPARATOR)?;
		}

		return Ok(());
	}

	let half: LogDepth = FORMAT_FULL_MAX_DEPTH / 2;

	for _ in 0..half {
		write!(out, "{}", FORMAT_FULL_DEPTH_SEPARATOR)?;
	}
	write!(out, "{FORMAT_FULL_DEPTH_ELLIPSIS}")?;
	for _ in 0..FORMAT_FULL_MAX_DEPTH - half - 1 {
		write!(out, "{}", FORMAT_FULL_DEPTH_SEPARATOR)?;
	}

	Ok(())
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::ColorFull`] into a [`io::Write`].
pub fn write<T: io::Write>(out: &mut T, buf: &mut Vec<u8>, delimiter: &Vec<u8>, time_format: &Format, update: &LogUpdate) -> io::Result<()> {
	// construct header and measure its lenght to properly align all log output lines
	// TODO: rework once ntime returns proper time format length
	buf.clear();
	update.when().write(buf, time_format)?;

	write!(buf, " [{level:<LEVEL_LONG_NAME_MAX_LENGTH$}]", level = update.level().as_long_str(),)?;

	write_depth_spacer(buf, *update.depth())?;

	let header_len = buf.len();
	out.write(buf)?;

	// output fixed attributes on the first line, if any...
	let mut wrote: bool = false;
	for (key, val, meta) in update.attributes().iter() {
		if !meta.get(MetadataField::Ephemeral) {
			write!(out, " {key}=")?;
			write_value(out, update.attributes(), &val)?;
			wrote = true;
		}
	}
	if wrote {
		out.write(delimiter.as_slice())?;
		write!(out, "{:header_len$}", "")?;
	}

	// ...ephemeral attributes on a second line, if any...
	wrote = false;
	for (key, val, meta) in update.attributes().iter() {
		if meta.get(MetadataField::Ephemeral) {
			write!(out, " {key}=")?;
			write_value(out, update.attributes(), &val)?;
			wrote = true;
		}
	}
	if wrote {
		out.write(delimiter.as_slice())?;
		write!(out, "{:header_len$}", "")?;
	}

	// ...then the message body.
	write!(out, " {msg}", msg = update.message(),)?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::attributes::{Scalar, Value};
	use crate::level::Level;
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
	fn serialize() {
		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123 as i32));
		attrs.insert_ephemeral("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert_ephemeral("a_set", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));

		let ts = Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp");

		for tc in [
			(
				PartialLogUpdate::new(ts.clone(), Level::Warning, 0, String::from("test full, no depth")),
				"1776016599123000456 [WARNING] an_int=123 some_string=\"hi there!\"
                              a_float=-456.789 a_set=[0x14da0eb6, true]
                              test full, no depth",
			),
			(
				PartialLogUpdate::new(ts.clone(), Level::Info, 3, String::from("test full, half depth")),
				"1776016599123000456 [INFO   ]          an_int=123 some_string=\"hi there!\"
                                       a_float=-456.789 a_set=[0x14da0eb6, true]
                                       test full, half depth",
			),
			(
				PartialLogUpdate::new(ts.clone(), Level::Panic, 7, String::from("test full, over max depth")),
				"1776016599123000456 [PANIC  ]      ...       an_int=123 some_string=\"hi there!\"
                                             a_float=-456.789 a_set=[0x14da0eb6, true]
                                             test full, over max depth",
			),
		] {
			let (pupdate, want) = tc;

			let mut buf: Vec<u8> = Vec::new();
			let delimiter: Vec<u8> = [b'\n'].to_vec();

			let mut out = Vec::new();

			let update = LogUpdate::from((&pupdate, &attrs));
			let time_format = &ntime::Format::TimestampNanoseconds;

			assert!(write(&mut out, &mut buf, &delimiter, time_format, &update).is_ok());
			assert_eq!(String::from_utf8(out).unwrap(), String::from(want));
		}
	}
}
