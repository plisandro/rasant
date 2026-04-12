//! Formatting module for log writes, given ([`LogUpdate`] + attributes).
mod color_compact;
mod compact;
mod json;

use ntime;
use std::io;

use crate::attributes;
use crate::constant::{ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP};
use crate::sink::LogUpdate;

/// Supported log output format for all sinks.
#[derive(Clone, Debug)]
pub enum OutputFormat {
	/// A compact string: `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2`
	Compact,
	/// A compact colored string, for terminals supporting standard [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code): `2026-01-02 15:16:17.890 INF some log message key_1=value_1 key2=value_2`
	ColorCompact,
	/// A JSON-formatted string entry: `{"timestamp":123456,"level":"info","message":"some log message","key_1":"=value_1","key_2":"=value_2"}`
	Json,
}

/// Formatting errors.
#[derive(Clone, Debug)]
pub enum FormatterError {
	DelimiterNotAString,
}

impl OutputFormat {
	/// Returns a name for an `OutputFormat`.
	pub fn name(&self) -> String {
		match self {
			Self::Compact => "compact",
			Self::ColorCompact => "compact (w/console color)",
			Self::Json => "JSON",
		}
		.into()
	}
}

/// Configuration struct for output formatting.
#[derive(Clone, Debug)]
pub struct FormatterConfig {
	/// Output formatting configuration.
	pub format: OutputFormat,
	/// Time format for log entries, as [`ntime::Format`].
	pub time_format: ntime::Format,
	/// A separator for log entries, as a slice of [`u8`]s.
	pub delimiter: Vec<u8>,
}

impl FormatterConfig {
	/// Returns the default [`FormatterConfig`], which is [`OutputFormat::Compact`] with date/time + milliseconds in local timezone.
	pub fn default() -> Self {
		compact::default_format_config()
	}

	/// Returns a default [`FormatterConfig`] for [`OutputFormat::Compact`], with date/time + milliseconds in local timezone.
	pub fn default_compact() -> Self {
		compact::default_format_config()
	}

	/// Returns a default [`FormatterConfig`] for color [`OutputFormat::ColorCompact`], with date/time + milliseconds in local timezone.
	pub fn default_color() -> Self {
		color_compact::default_format_config()
	}

	/// Returns a default [`FormatterConfig`] for [`OutputFormat::Json`], with times as milliseconds since UNIX epoch.
	pub fn default_json() -> Self {
		json::default_format_config()
	}
}

/// Serializes and writes log updates + attributes.
#[derive(Clone, Debug)]
pub struct Formatter {
	format: OutputFormat,
	time_key: String,
	time_format: ntime::Format,
	delimiter: Vec<u8>,
}

impl Formatter {
	/// Initializes a [`Formatter`] from a given [`FormatterConfig`]
	pub fn new(conf: FormatterConfig) -> Self {
		Self {
			format: conf.format,
			time_key: match &conf.time_format {
				ntime::Format::TimestampSeconds | ntime::Format::TimestampMilliseconds => String::from(ATTRIBUTE_KEY_TIMESTAMP),
				_ => String::from(ATTRIBUTE_KEY_TIME),
			},
			time_format: conf.time_format,
			delimiter: conf.delimiter,
		}
	}

	/// Writes a formatted [`LogUpdate`] + attributes ['Map`] into a [`io::Write`].
	pub fn write<T: io::Write>(&self, out: &mut T, update: &LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		match self.format {
			OutputFormat::Compact => compact::write(out, &self.time_format, update, attrs),
			OutputFormat::ColorCompact => color_compact::write(out, &self.time_format, update, attrs),
			OutputFormat::Json => json::write(out, &self.time_format, &self.time_key, &update, attrs),
		}
	}

	/// Write a formatted delimiter into a [`io::Write`].
	pub fn write_delimiter<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		match out.write(self.delimiter.as_slice()) {
			Ok(_) => Ok(()),
			Err(e) => Err(e),
		}
	}

	/// Serializes a formatted [`LogUpdate`] + attributes ['Map`] into a [`String`].
	pub fn as_string(&self, update: &LogUpdate, attrs: &attributes::Map) -> String {
		let mut out = Vec::new();

		match self.write(&mut out, update, attrs) {
			Ok(_) => (),
			Err(e) => panic!("failed to convert log update {update:?} to string buffer: {e}"),
		};
		match String::from_utf8(out) {
			Ok(s) => s,
			Err(e) => panic!("failed to convert log update {update:?} to UTF8: {e}"),
		}
	}

	/// Serializes a formatted delimiter into a [`String`].
	pub fn delimiter_as_string(&self) -> Result<String, FormatterError> {
		match String::from_utf8(self.delimiter.clone()) {
			Ok(s) => Ok(s),
			Err(_) => Err(FormatterError::DelimiterNotAString),
		}
	}
}

/// Returns a formatted string for a [`LogUpdate`] + attributes ['Map`], suitable for use with ['panic!`].
pub fn as_panic_string(update: &LogUpdate, attrs: &attributes::Map) -> String {
	let formatter = Formatter::new(FormatterConfig {
		format: OutputFormat::Compact,
		..FormatterConfig::default_compact()
	});
	formatter.as_string(update, attrs)
}
