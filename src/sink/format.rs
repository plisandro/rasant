use std::io;

use crate::console::Color;
use crate::level::Level;
use crate::sink::LogUpdate;
use crate::sink::attributes::{KEY_ERROR, KEY_MESSAGE, KEY_TIME, KEY_TIMESTAMP};
use crate::time;

#[derive(Clone, Debug)]
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
		}
	}

	fn format_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		// "2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2"

		// build output header
		update.when.write(out, &self.time_format)?;
		write!(out, " [{level}] {msg}", level = update.level.as_short_str(), msg = update.msg)?;

		// append fields
		for (key, val) in update.attributes.into_iter() {
			write!(out, " {key}=")?;
			val.write_quoted(out)?;
		}

		Ok(())
	}

	fn format_color_compact<T: io::Write>(&self, out: &mut T, update: &LogUpdate) -> io::Result<()> {
		// "2026-01-02 15:16:17.890 [INF] some log message key_1=value_1 key2=value_2"

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
		for (key, val) in update.attributes.into_iter() {
			write!(
				out,
				" {key_open}{key}{key_close}={val_open}",
				key_open = Color::Cyan.to_escape_str(),
				key_close = Color::Default.to_escape_str(),
				// error attributes are highlighted in red
				val_open = if key == KEY_ERROR { Color::BrightRed.to_escape_str() } else { "" }
			)?;
			val.write_quoted(out)?;
			write!(out, "{val_close}", val_close = Color::Default.to_escape_str())?;
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
		for (key, val) in update.attributes.into_iter() {
			write!(out, ",\"{key}\":")?;
			val.write_json(out)?;
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
