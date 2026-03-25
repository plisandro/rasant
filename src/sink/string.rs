use std::io;
use std::string;
use std::sync::Arc;

use crate::attributes;
use crate::sink::format;
use crate::{sink, time};

use std::sync::Mutex;

pub struct StringConfig {
	pub type_str: string::String,
	pub line_delimiter: string::String,
	pub formatter_cfg: format::FormatterConfig,
	pub mock_time: bool,
	pub mock_logger_id: bool,
}

impl Default for StringConfig {
	fn default() -> Self {
		Self {
			type_str: "default".into(),
			formatter_cfg: format::FormatterConfig {
				time_format: time::StringFormat::UtcMillisDateTime,
				..format::FormatterConfig::default()
			},
			line_delimiter: "\n".into(),
			mock_time: false,
			mock_logger_id: false,
		}
	}
}

// String string sink, useful mostly for testing.
pub struct String {
	name: string::String,
	formatter: format::Formatter,
	line_delimiter: string::String,
	out: Arc<Mutex<string::String>>,
	frozen_logger_id: Option<u32>,
	frozen_now: Option<time::Timestamp>,
	frozen_now_tick: Option<time::Duration>,
}

impl String {
	pub fn new(conf: StringConfig) -> Self {
		Self {
			name: format!("{} log string", conf.type_str),
			formatter: format::Formatter::new(conf.formatter_cfg),
			line_delimiter: conf.line_delimiter,
			out: Arc::new(Mutex::new(string::String::new())),
			frozen_logger_id: if conf.mock_logger_id { Some(100 as u32) } else { None },
			frozen_now: if conf.mock_time {
				// 2026-03-04 15:10:15 GMT
				Some(time::Timestamp::from_secs(1772637015))
			} else {
				None
			},
			frozen_now_tick: if conf.mock_time { Some(time::Duration::from_millis(1234)) } else { None },
		}
	}

	pub fn output(&self) -> Arc<Mutex<string::String>> {
		self.out.clone()
	}

	pub fn reset(&self) {
		self.out.lock().unwrap().clear();
	}
}

impl sink::Sink for String {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		let mut out = match self.out.lock() {
			Ok(s) => s,
			Err(e) => {
				panic!("failed to acquire lock for log string: {e}");
			}
		};

		let line = if self.frozen_now.is_some() || self.frozen_logger_id.is_some() {
			// apply mocks
			let mut nupdate = update.clone();
			let mut mock_attrs: Option<attributes::Map> = None;

			if let Some(t) = self.frozen_now.as_mut() {
				nupdate.when = t.clone();
			}
			if let Some(id) = self.frozen_logger_id {
				if attrs.has(attributes::KEY_LOGGER_ID) {
					mock_attrs = Some(attrs.clone());
					mock_attrs.as_mut().unwrap().insert(attributes::KEY_LOGGER_ID, id);
					self.frozen_logger_id = Some(id + 1);
				};
			}

			self.formatter.as_string(
				&nupdate,
				match mock_attrs.as_ref() {
					Some(a) => a,
					None => attrs,
				},
			)
		} else {
			self.formatter.as_string(update, attrs)
		};

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
}
