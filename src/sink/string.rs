use std::io;
use std::string;

use crate::sink::format;
use crate::{sink, time};

use std::sync::Mutex;

pub struct StringConfig<'s> {
	pub type_str: string::String,
	pub line_delimiter: string::String,
	pub formatter_cfg: format::FormatterConfig,
	pub mock_time: bool,
	pub out: Option<&'s Mutex<string::String>>,
}

impl<'s> Default for StringConfig<'s> {
	fn default() -> Self {
		Self {
			type_str: "default".into(),
			formatter_cfg: format::FormatterConfig {
				time_format: time::StringFormat::UtcMillisDateTime,
				..format::FormatterConfig::default()
			},
			line_delimiter: "\n".into(),
			mock_time: false,
			out: None,
		}
	}
}

// String string sink, useful mostly for testing.
pub struct String<'s> {
	name: string::String,
	formatter: format::Formatter,
	line_delimiter: string::String,
	out: &'s Mutex<string::String>,
	frozen_now: Option<time::Timestamp>,
	frozen_now_tick: Option<time::Duration>,
}

impl<'s> String<'s> {
	pub fn new(conf: StringConfig<'s>) -> Self {
		let Some(out) = conf.out else {
			panic!("missing sink String for string logger");
		};

		Self {
			name: format!("{} log string", conf.type_str),
			formatter: format::Formatter::new(conf.formatter_cfg),
			line_delimiter: conf.line_delimiter,
			out: out,
			frozen_now: if conf.mock_time {
				// 2026-03-04 15:10:15 GMT
				Some(time::Timestamp::from_secs(1772637015))
			} else {
				None
			},
			frozen_now_tick: if conf.mock_time { Some(time::Duration::from_millis(1234)) } else { None },
		}
	}
}

impl sink::Sink for String<'_> {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate) -> io::Result<()> {
		let mut out = match self.out.lock() {
			Ok(s) => s,
			Err(e) => {
				panic!("failed to acquire lock for log string: {e}");
			}
		};

		let line: string::String;
		if let Some(now) = &self.frozen_now {
			line = self.formatter.as_string(&(update.with_when(now)));
		} else {
			line = self.formatter.as_string(&update);
		}

		if !out.is_empty() {
			out.push_str(&self.line_delimiter);
		}
		out.push_str(&line);

		if let Some(now) = &mut (self.frozen_now) {
			if let Some(tick) = self.frozen_now_tick {
				now.add_duration(&tick);
			}
		}

		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}

	fn drop(&self) {}
}
