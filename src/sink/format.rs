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

pub struct FormatterConfig {
	pub output: OutputFormat,
	pub time_format: time::StringFormat,
}

impl FormatterConfig {
	pub fn default() -> Self {
		Self {
			output: OutputFormat::Compact,
			time_format: time::StringFormat::LocalMillisDateTime,
		}
	}

	pub fn json() -> Self {
		Self {
			output: OutputFormat::Json,
			time_format: time::StringFormat::TimestampSeconds,
		}
	}
}

pub struct Formatter {
	output: OutputFormat,
	time_format: time::StringFormat,
}

impl Formatter {
	pub fn new(conf: FormatterConfig) -> Self {
		Self {
			output: conf.output,
			time_format: conf.time_format,
		}
	}

	fn time_kv(&self, update: &LogUpdate) -> (&str, String) {
		let time_key = match self.time_format {
			time::StringFormat::TimestampSeconds | time::StringFormat::TimestampMilliseconds => KEY_TIMESTAMP,
			_ => KEY_TIME,
		};
		let time_str = update.when.as_string(&self.time_format);

		(time_key, time_str)
	}

	fn format_compact(&self, update: &LogUpdate) -> String {
		let (_, time_str) = self.time_kv(update);

		// build output header
		let mut out = format!("{time_str} [{level}] {msg}", time_str = time_str, level = update.level.as_short_str(), msg = update.msg);

		// append fields
		for k in update.attributes.keys() {
			out += format!(" {key}={val}", key = k, val = update.attributes.get_as_quoted_string(k),).as_str();
		}

		out
	}

	fn format_color_compact(&self, update: &LogUpdate) -> String {
		let (_, time_str) = self.time_kv(update);

		// build output header
		let msg = if Level::Debug.includes(&update.level) {
			&update.msg
		} else {
			// update messages above debug are highlighted in white
			&Color::BrightWhite.paint(update.msg.as_str())
		};

		let mut out = format!(
			"{time_str} {level} {msg}",
			time_str = Color::BrightBlack.paint(time_str.as_str()),
			level = update.level.as_color_short_str(),
			msg = msg,
		);

		// append fields
		for k in update.attributes.keys() {
			let val = if k == KEY_ERROR {
				// error attributes are highlighted in red
				Color::BrightRed.paint(update.attributes.get_as_quoted_string(k).as_str())
			} else {
				update.attributes.get_as_quoted_string(k)
			};

			out += format!(" {key}={val}", key = Color::Cyan.paint(k), val = val,).as_str();
		}

		out
	}

	fn format_json(&self, update: &LogUpdate) -> String {
		let (time_key, mut time_str) = self.time_kv(update);

		if !self.time_format.is_numeric() {
			time_str.insert_str(0, "\"");
			time_str.push_str("\"");
		}

		// build output header
		let mut out: String = format!(
			"{{\"{time_key}\":{time_str},\"level\":\"{level}\",\"{msg_key}\":\"{msg}\"",
			time_key = time_key,
			time_str = time_str,
			level = update.level.as_str(),
			msg_key = KEY_MESSAGE,
			msg = update.msg,
		);

		// append fields
		for k in update.attributes.keys() {
			out += format!(",\"{key}\":{val}", key = k, val = update.attributes.get_as_json_string(k)).as_str();
		}
		out += "}";

		out
	}

	// TODO: replace string output with buffer writes for performance
	pub fn format(&self, update: &LogUpdate) -> String {
		match self.output {
			OutputFormat::Compact => self.format_compact(update),
			OutputFormat::ColorCompact => self.format_color_compact(update),
			//OutputFormat::Long => self.format_long(update),
			OutputFormat::Json => self.format_json(&update),
		}
	}
}
