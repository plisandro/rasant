use std::fmt;
use std::io;

use crate::console::Color;
use crate::level::Level;
use crate::sink::LogUpdate;
use crate::sink::attributes::{KEY_ERROR, KEY_MESSAGE, KEY_TIME, KEY_TIMESTAMP};
use crate::time;

#[derive(Debug)]
pub enum OutputFormat {
	Compact,
	ColorCompact,
	//Long,
	Json,
}

impl OutputFormat {
	pub fn name(&self) -> String {
		match self {
			Self::Compact => "compact",
			Self::ColorCompact => "compact (w/console color)",
			Self::Json => "JSON",
		}
		.into()
	}
}

pub struct FormatterConfig {
	pub format: OutputFormat,
	pub time_format: time::StringFormat,
}

impl FormatterConfig {
	pub fn default() -> Self {
		Self {
			format: OutputFormat::Compact,
			time_format: time::StringFormat::LocalMillisDateTime,
		}
	}

	pub fn json() -> Self {
		Self {
			format: OutputFormat::Json,
			time_format: time::StringFormat::TimestampSeconds,
		}
	}
}

pub struct Formatter {
	format: OutputFormat,
	time_key: String,
	time_format: time::StringFormat,
	output_buffer: io::Cursor<Vec<u8>>,
}

impl Formatter {
	pub fn new(conf: FormatterConfig) -> Self {
		Self {
			format: conf.format,
			time_key: match &conf.time_format {
				time::StringFormat::TimestampSeconds | time::StringFormat::TimestampMilliseconds => String::from(KEY_TIMESTAMP),
				_ => String::from(KEY_TIME),
			},
			time_format: conf.time_format,
			output_buffer: io::Cursor::new(Vec::new()),
		}
	}

	fn format_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		// "2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2"

		// build output header
		update.when.write(out, &self.time_format)?;
		write!(out, " [{level}] {msg}", level = update.level.as_short_str(), msg = update.msg)?;

		// append fields
		for k in update.attributes.keys() {
			write!(out, " {key}={val}", key = k, val = update.attributes.get_as_quoted_string(k),)?;
		}

		Ok(())
	}

	fn format_color_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		// "2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2"

		// build output header
		let msg = if Level::Debug.includes(&update.level) {
			&update.msg
		} else {
			// update messages above debug are highlighted in white
			&Color::BrightWhite.paint(update.msg.as_str())
		};

		update.when.write(out, &self.time_format)?;
		write!(out, " {level} {msg}", level = update.level.as_color_short_str(), msg = msg,)?;

		// append fields
		for k in update.attributes.keys() {
			let val = if k == KEY_ERROR {
				// error attributes are highlighted in red
				Color::BrightRed.paint(update.attributes.get_as_quoted_string(k).as_str())
			} else {
				update.attributes.get_as_quoted_string(k)
			};

			write!(out, " {key}={val}", key = Color::Cyan.paint(k), val = val,)?;
		}

		Ok(())
	}

	fn format_json<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		// "{"timestamp":123456,"level":"info","message":"some log message","key_1":"=value_1","key_2":"=value_2"}"

		// build output header
		write!(
			out,
			"{{\"{time_key}\":{time_delimiter}",
			time_key = self.time_key,
			time_delimiter = if self.time_format.is_numeric() { "" } else { "\"" },
		)?;
		update.when.write(out, &self.time_format)?;
		write!(
			out,
			"{time_delimiter},\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
			time_delimiter = if self.time_format.is_numeric() { "" } else { "\"" },
			level = update.level.as_str(),
			msg_key = KEY_MESSAGE,
			msg = update.msg,
		)?;

		// append fields
		for k in update.attributes.keys() {
			write!(out, ",\"{key}\":{val}", key = k, val = update.attributes.get_as_json_string(k))?;
		}
		write!(out, "}}")?;

		Ok(())
	}

	pub fn write<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		match self.format {
			OutputFormat::Compact => self.format_compact(out, update),
			OutputFormat::ColorCompact => self.format_color_compact(out, update),
			//OutputFormat::Long => self.format_long(update),
			OutputFormat::Json => self.format_json(out, &update),
		}
	}

	pub fn as_string(&self, update: &LogUpdate) -> String {
		let mut out = io::Cursor::new(Vec::new());
		self.write(&mut out, update);

		match String::from_utf8(out.into_inner()) {
			Ok(s) => s,
			Err(e) => panic!("failed to convert log update {update:?} to string: {e}"),
		}
	}
}
