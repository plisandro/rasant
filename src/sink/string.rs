//! String logging [`sink`] module.
//!
//! String sinks are useful mostly for testing and, as a result, their focus is not
//! performance,  but usability.
//!
//! This sink writes all log updates into a [`std::sync::Mutex`]ed [`String`], which
//! can be accessed via public methods, and supports mocking a number of attributes
//! which can cause non-deterministic test results:
//!
//!   - If `mock_time` is `true`, time is pinned to a fixed start value, and
//!     increases monolithically with every log write.
//!   - If `mock_logger_id` is `true`, the `logger_id` atttibute is pinned to a
//!     fixed start value, and  increases monolithically with every log write.
//!
//! Unless you're writing tests, you _really_ want to use another [`sink`] type :)
use ntime;
use std::io;
use std::string;
use std::sync::Arc;

use crate::attributes;
use crate::attributes::ToValue;
use crate::constant::ATTRIBUTE_KEY_LOGGER_ID;
use crate::format;
use crate::sink;

use std::sync::Mutex;

/// Configuration struct for an [`String`] [`sink`].
pub struct StringConfig {
	/// A type string, used to define the sink's name.
	pub type_str: string::String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether to mock log update times.
	pub mock_time: bool,
	/// Whether to mock logger IDs.
	pub mock_logger_id: bool,
}

impl Default for StringConfig {
	fn default() -> Self {
		Self {
			type_str: "default".into(),
			formatter_cfg: format::FormatterConfig {
				time_format: ntime::Format::UtcMillisDateTime,
				..format::FormatterConfig::default()
			},
			mock_time: false,
			mock_logger_id: false,
		}
	}
}

/// String logging [`sink`] definition.
pub struct String {
	name: string::String,
	formatter: format::Formatter,
	out: Arc<Mutex<string::String>>,
	frozen_logger_id: Option<u32>,
	frozen_now: Option<ntime::Timestamp>,
	frozen_now_tick: Option<ntime::Duration>,
	delimiter: string::String,
}

impl String {
	/// Initializes a string [`sink`] from a [`StringConfig`].
	pub fn new(conf: StringConfig) -> Self {
		let formatter = format::Formatter::new(conf.formatter_cfg);
		let delimiter = match formatter.delimiter_as_string() {
			Ok(s) => s,
			Err(e) => panic!("cannot format delimiter for String sink: {e:?}"),
		};

		Self {
			name: format!("{} log string", conf.type_str),
			formatter: formatter,
			out: Arc::new(Mutex::new(string::String::new())),
			frozen_logger_id: if conf.mock_logger_id { Some(100 as u32) } else { None },
			frozen_now: if conf.mock_time {
				// 2026-03-04 15:10:15 GMT
				Some(ntime::Timestamp::from_secs(1772637015))
			} else {
				None
			},
			frozen_now_tick: if conf.mock_time { Some(ntime::Duration::from_millis(1234)) } else { None },
			delimiter: delimiter,
		}
	}

	/// Returns the underlying [`String`] buffer for this sink.
	pub fn output(&self) -> Arc<Mutex<string::String>> {
		self.out.clone()
	}

	/// Clears the underlying [`String`] buffer for this sink.
	pub fn clear(&self) {
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
				if attrs.has(ATTRIBUTE_KEY_LOGGER_ID) {
					mock_attrs = Some(attrs.clone());
					mock_attrs.as_mut().unwrap().insert_ephemeral(ATTRIBUTE_KEY_LOGGER_ID, id.to_value());
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
			*out += &self.delimiter;
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
