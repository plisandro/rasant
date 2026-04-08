//! Formatting module for log writes, given ([`LogUpdate`] + attributes).
use ntime;
use std::io;

use crate::attributes;
use crate::console::Color;
use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP, DEFAULT_LOG_DELIMITER_STRING};
use crate::level::Level;
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
	/// Returns a default [`FormatterConfig`] for text: [`OutputFormat::Compact`] with date/time + milliseconds in local timezone).
	pub fn default() -> Self {
		Self {
			format: OutputFormat::Compact,
			time_format: ntime::Format::LocalMillisDateTime,
			delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
		}
	}

	/// Returns a default [`FormatterConfig`] for color text: [`OutputFormat::ColorCompact`] with date/time + milliseconds in local timezone.
	pub fn color() -> Self {
		Self {
			format: OutputFormat::ColorCompact,
			time_format: ntime::Format::LocalMillisDateTime,
			delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
		}
	}

	/// Returns a default [`FormatterConfig`] for JSON: [`OutputFormat::Json`] with times as milliseconds since UNIX epoch.
	pub fn json() -> Self {
		Self {
			format: OutputFormat::Json,
			time_format: ntime::Format::TimestampMilliseconds,
			delimiter: DEFAULT_LOG_DELIMITER_STRING.into(),
		}
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

	// Compact formatter: `2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2`
	fn format_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		// build output header
		update.when.write(out, &self.time_format)?;
		write!(out, " [{level}] {msg}", level = update.level.as_short_str(), msg = update.msg)?;

		// append fields
		for (key, val) in attrs.into_iter() {
			write!(out, " {key}=")?;
			val.write_quoted(out)?;
		}

		Ok(())
	}

	// Compact color formatter: `2026-01-02 15:16:17.890 INF some log message key_1=value_1 key2=value_2`
	fn format_color_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		// update messages above debug are highlighted in white
		let msg_color = if Level::Debug.includes(&update.level) { Color::Default } else { Color::BrightWhite };
		let level_color = update.level.color();

		update.when.write(out, &self.time_format)?;
		write!(
			out,
			" {level_open}{level}{level_close} {msg_open}{msg}{msg_close}",
			level_open = level_color.to_escape_str(),
			level = update.level.as_short_str(),
			level_close = Color::Default.to_escape_str(),
			msg_open = msg_color.to_escape_str(),
			msg = update.msg,
			msg_close = Color::Default.to_escape_str(),
		)?; // update messages above debug are highlighted in white

		// append fields
		for (key, val) in attrs.into_iter() {
			write!(
				out,
				" {key_open}{key}{key_close}={val_open}",
				key_open = Color::Cyan.to_escape_str(),
				key_close = Color::Default.to_escape_str(),
				// error attributes are highlighted in red
				val_open = if key == ATTRIBUTE_KEY_ERROR { Color::BrightRed.to_escape_str() } else { "" }
			)?;
			val.write_quoted(out)?;
			write!(out, "{val_close}", val_close = Color::Default.to_escape_str())?;
		}

		Ok(())
	}

	// JSON formatter: `{"timestamp":123456,"level":"info","message":"some log message","key_1":"=value_1","key_2":"=value_2"}`
	fn format_json<T: io::Write>(&self, out: &mut T, update: &LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		// build output header
		match self.time_format.as_integer(&update.when) {
			Some(timestamp_int) => write!(
				out,
				"{{\"{time_key}\":{timestamp_int},\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
				time_key = self.time_key,
				level = update.level.as_str(),
				msg_key = ATTRIBUTE_KEY_MESSAGE,
				msg = update.msg,
			)?,
			None => {
				write!(out, "{{\"{time_key}\":\"", time_key = self.time_key)?;
				update.when.write(out, &self.time_format)?;
				write!(
					out,
					"\",\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
					level = update.level.as_str(),
					msg_key = ATTRIBUTE_KEY_MESSAGE,
					msg = update.msg,
				)?;
			}
		}

		// append fields
		for (key, val) in attrs.into_iter() {
			write!(out, ",\"{key}\":")?;
			val.write_json(out)?;
		}
		write!(out, "}}")?;

		Ok(())
	}

	/// Writes a formatted [`LogUpdate`] + attributes ['Map`] into a [`io::Write`].
	pub fn write<T: io::Write>(&self, out: &mut T, update: &LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		match self.format {
			OutputFormat::Compact => self.format_compact(out, update, attrs),
			OutputFormat::ColorCompact => self.format_color_compact(out, update, attrs),
			OutputFormat::Json => self.format_json(out, &update, attrs),
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
		..FormatterConfig::default()
	});
	formatter.as_string(update, attrs)
}
