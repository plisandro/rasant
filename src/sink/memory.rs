//! Memory buffer logging [sink] module.
//!
//! Buffer sinks are useful mostly for testing and, as a result, their focus is
//! not performance, but usability.
//!
//! This sink writes all log updates into a [`Vec<u8>`], and supports mocking a
//! number of attributes which can cause non-deterministic test results:
//!
//!   - If `mock_time` is `true`, time is pinned to a fixed start value, and
//!     increases monolithically with every log write.
//!   - If `mock_logger_id` is `true`, the `logger_id` atttibute is pinned to a
//!     fixed start value, and  increases monolithically with every log write.
//!
//! Output is encapsulated in [`MemoryOutput`] instances, which supports casting
//! to common types such as [`String`].
//!
//! Unless you're writing tests, you _really_ want to use another [sink] type :)

use ntime;
use std::io;
use std::sync::{Arc, Mutex};

use crate::attributes;
use crate::constant::ATTRIBUTE_KEY_LOGGER_ID;
use crate::format;
use crate::sink;

/// Configuration struct for an [`Memory`] [`sink`].
pub struct MemoryConfig {
	/// A type string, used to define the sink's name.
	pub type_str: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether to mock log update times.
	pub mock_time: bool,
	/// Whether to mock logger IDs.
	pub mock_logger_id: bool,
}

/// Container for [`Memory`] [`sink`] output.
pub struct MemoryOutput {
	/// [`std::sync::Arc`]ed and [`std::sync::Mutex`]ed [`Vec<u8>`] output buffer.
	out: Arc<Mutex<Vec<u8>>>,
}

impl MemoryOutput {
	fn new(out: &Arc<Mutex<Vec<u8>>>) -> Self {
		Self { out: out.clone() }
	}

	/// Returns the buffer contents as a [`Vec<u8>`].
	pub fn as_bytes(&self) -> Vec<u8> {
		self.out.lock().unwrap().clone()
	}

	/// Returns the buffer contents as a [`String`].
	pub fn as_string(&self) -> String {
		String::from_utf8(self.out.lock().unwrap().clone()).expect("invalid UTF-8 contents for Memory sink")
	}
}

impl Default for MemoryConfig {
	fn default() -> Self {
		Self {
			type_str: "default".into(),
			formatter_cfg: format::FormatterConfig {
				time_format: ntime::Format::UtcMillisDateTime,
				delimiter: vec![b'\n'],
				..format::FormatterConfig::default()
			},
			mock_time: false,
			mock_logger_id: false,
		}
	}
}

/// Byte buffer logging [`sink`] definition.
pub struct Memory {
	name: String,
	formatter: format::Formatter,
	out: Arc<Mutex<Vec<u8>>>,
	frozen_logger_id: Option<u32>,
	frozen_now: Option<ntime::Timestamp>,
	frozen_now_tick: Option<ntime::Duration>,
	mock_attributes: attributes::Map,
}

impl Memory {
	/// Initializes an in-[`Memory`] byte buffer [`sink`] from a [`MemoryConfig`].
	pub fn new(conf: MemoryConfig) -> Self {
		let formatter = format::Formatter::new(conf.formatter_cfg);

		Self {
			name: format!("{} log string", conf.type_str),
			formatter: formatter,
			out: Arc::new(Mutex::new(Vec::new())),
			frozen_logger_id: if conf.mock_logger_id { Some(100 as u32) } else { None },
			frozen_now: if conf.mock_time {
				// 2026-03-04 15:10:15 GMT
				Some(ntime::Timestamp::from_secs(1772637015))
			} else {
				None
			},
			frozen_now_tick: if conf.mock_time { Some(ntime::Duration::from_millis(1234)) } else { None },
			mock_attributes: attributes::Map::new(),
		}
	}

	/// Returns a [`MemoryOutput`] with contents for the sink.
	pub fn output(&self) -> MemoryOutput {
		MemoryOutput::new(&self.out)
	}

	/// Clears the underlying [`&[u8]`] buffer for this sink.
	pub fn clear(&mut self) {
		self.out.lock().unwrap().clear();
	}
}

impl sink::Sink for Memory {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		let mut out = self.out.lock().unwrap();

		let entry = if self.frozen_now.is_some() || self.frozen_logger_id.is_some() {
			// apply mocks
			let mut mock_update = update.clone();
			self.mock_attributes.copy_from(attrs);

			if let Some(t) = self.frozen_now.as_mut() {
				mock_update.when = t.clone();
			}
			if let Some(id) = self.frozen_logger_id {
				if self.mock_attributes.has(ATTRIBUTE_KEY_LOGGER_ID) {
					self.mock_attributes.insert(ATTRIBUTE_KEY_LOGGER_ID, attributes::Value::from(id));
					self.frozen_logger_id = Some(id + 1);
				};
			}

			self.formatter.as_bytes(&mock_update, &self.mock_attributes)
		} else {
			self.formatter.as_bytes(update, attrs)
		};

		if !out.is_empty() {
			out.extend_from_slice(self.formatter.delimiter());
		}
		out.extend(entry);

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

/// Returns an initialized memory buffer [sink][`crate::sink`], with default values.
pub fn default() -> Memory {
	Memory::new(MemoryConfig::default())
}
