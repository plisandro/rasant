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
use crate::attributes::{MetadataField, MetadataImpl};
use crate::constant::{
	DEFAULT_LOG_DELIMITER_STRING, FORMAT_FULL_DEPTH_ELLIPSIS, FORMAT_FULL_DEPTH_ELLIPSIS_LENGTH, FORMAT_FULL_DEPTH_SEPARATOR, FORMAT_FULL_DEPTH_SEPARATOR_LENGTH, FORMAT_FULL_MAX_DEPTH,
	LEVEL_LONG_NAME_MAX_LENGTH,
};
use crate::format::compact;
use crate::format::{FormatterConfig, OutputFormat};
use crate::sink::{LogDepth, LogUpdate};
use crate::write::IoWriter;

/// Returns a default [`FormatterConfig`] for [`OutputFormat::Full`].
pub fn default_format_config() -> FormatterConfig {
	FormatterConfig {
		format: OutputFormat::Full,
		time_format: ntime::Format::DateTime(ntime::Timezone::Local, ntime::Precision::Milliseconds),
		delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
	}
}

// Serializes a [`Value`] for [`OutputFormat::ColorFull`] into a [`io::Write`].
fn write_value(out: &mut IoWriter, update: &LogUpdate, val: &Value) -> io::Result<()> {
	compact::write_value(out, update, val)
}

// Compute spacer length based on the [`LogDepth`] for a [`LogUpdate`].
fn depth_spacer_len(depth: LogDepth) -> usize {
	if depth <= FORMAT_FULL_MAX_DEPTH {
		return depth as usize * FORMAT_FULL_DEPTH_SEPARATOR_LENGTH;
	}

	FORMAT_FULL_DEPTH_ELLIPSIS_LENGTH + (FORMAT_FULL_MAX_DEPTH - 1) as usize * FORMAT_FULL_DEPTH_SEPARATOR_LENGTH
}

// Write a spacer based on the [`LogDepth`] for a [`LogUpdate`].
fn write_depth_spacer(out: &mut IoWriter, depth: LogDepth) -> io::Result<()> {
	if depth <= FORMAT_FULL_MAX_DEPTH {
		for _ in 0..depth {
			out.str(FORMAT_FULL_DEPTH_SEPARATOR)?;
		}

		return Ok(());
	}

	let half: LogDepth = FORMAT_FULL_MAX_DEPTH / 2;

	for _ in 0..half {
		out.str(FORMAT_FULL_DEPTH_SEPARATOR)?;
	}
	out.str(FORMAT_FULL_DEPTH_ELLIPSIS)?;
	for _ in 0..FORMAT_FULL_MAX_DEPTH - half - 1 {
		out.str(FORMAT_FULL_DEPTH_SEPARATOR)?;
	}

	Ok(())
}

/// Serializes a [`LogUpdate`] as [`OutputFormat::ColorFull`] into a [`io::Write`].
pub fn write(mut out: &mut IoWriter, delimiter: &Vec<u8>, time_format: &Format, update: &LogUpdate) -> io::Result<()> {
	// compute header length to properly align all log output lines
	// "<time str> [WARNING]" + <depth spacer>
	let header_len: usize = update.when().string_len_as(time_format) + 10 + depth_spacer_len(*update.depth());

	// build output header: "timestamp [level] message"
	update.when().write(&mut out, time_format)?;
	out.str(" [")?;
	out.str_right_padded(update.level().as_long_str(), b' ', LEVEL_LONG_NAME_MAX_LENGTH)?;
	out.u8(b']')?;
	write_depth_spacer(out, *update.depth())?;

	// output fixed attributes on the first line, if any...
	let mut wrote: bool = false;
	for (key, val, meta) in update.attribute_full_iter() {
		if !meta.get(MetadataField::Ephemeral) {
			out.space()?;
			out.str(key)?;
			out.u8(b'=')?;
			write_value(out, update, &val)?;
			wrote = true;
		}
	}
	if wrote {
		out.bytes(delimiter.as_slice())?;
		out.u8_repeated(b' ', header_len)?;
	}

	// ...ephemeral attributes on a second line, if any...
	wrote = false;
	for (key, val, meta) in update.attribute_full_iter() {
		if meta.get(MetadataField::Ephemeral) {
			out.space()?;
			out.str(key)?;
			out.u8(b'=')?;
			write_value(out, update, &val)?;
			wrote = true;
		}
	}
	if wrote {
		out.write_all(delimiter.as_slice())?;
		out.u8_repeated(b' ', header_len)?;
	}

	// ...then the message body.
	out.space()?;
	out.str(update.message())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use crate::attributes::{Map, Scalar, Value};
	use crate::level::Level;
	use ntime::Timestamp;

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
					Scalar::from(89801234567890123_usize),
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
			let mut out = IoWriter::new_bytes();

			let when = Timestamp::now();
			let attrs = Map::new();
			let update = LogUpdate::from((&when, &attrs));

			assert!(write_value(&mut out, &update, &v).is_ok());
			assert!(out.flush().is_ok());
			assert_eq!(out.into_string().unwrap(), want);
		}
	}

	#[test]
	fn serialize() {
		let mut attrs = Map::new();
		attrs.insert("an_int", Value::from(123_i32));
		attrs.insert_ephemeral("a_float", Value::from(-456.789));
		attrs.insert("some_string", Value::from("hi there!"));
		attrs.insert_ephemeral("a_set", Value::from(&[Scalar::from(349834934_usize), Scalar::from(true)]));
		attrs.insert("nothing", Value::from(None::<bool>));

		let when = Timestamp::try_from((2026, 4, 12, 17, 56, 39, 123, 456)).expect("failed to initialize timestamp");
		for tc in [
			(
				LogUpdate::from((&when, Level::Warning, 0, "test full, no depth", &attrs)),
				"1776016599123000456 [WARNING] an_int=123 some_string=\"hi there!\" nothing=<none>
                              a_float=-456.789 a_set=[0x14da0eb6, true]
                              test full, no depth",
			),
			(
				LogUpdate::from((&when, Level::Info, 3, "test full, half depth", &attrs)),
				"1776016599123000456 [INFO   ]          an_int=123 some_string=\"hi there!\" nothing=<none>
                                       a_float=-456.789 a_set=[0x14da0eb6, true]
                                       test full, half depth",
			),
			(
				LogUpdate::from((&when, Level::Panic, 7, "test full, over max depth", &attrs)),
				"1776016599123000456 [PANIC  ]      ...       an_int=123 some_string=\"hi there!\" nothing=<none>
                                             a_float=-456.789 a_set=[0x14da0eb6, true]
                                             test full, over max depth",
			),
		] {
			let (pupdate, want) = tc;

			let mut out = IoWriter::new_bytes();
			let delimiter: Vec<u8> = b"\n".to_vec();

			let update = LogUpdate::from((&pupdate, &attrs));
			let time_format = &ntime::Format::Timestamp(ntime::Precision::Nanoseconds);

			assert!(write(&mut out, &delimiter, time_format, &update).is_ok());
			assert_eq!(out.into_string().unwrap(), String::from(want));
		}
	}
}
