use ntime::Timestamp;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::attributes;
use crate::attributes::value::{ToValue, Value};
use crate::format;
use crate::level::Level;
use crate::queue;
use crate::sink;
use crate::sink::Sink;

static GLOBAL_LOGGER_NEXT_UUID: Mutex<u32> = Mutex::new(0);

/// Base logger structure for Rasant.
pub struct Logger {
	id: u32,
	depth: sink::LogDepth,
	level: Level,
	async_writes: bool,
	attributes: attributes::Map,
	sinks: Vec<Arc<Mutex<Box<dyn Sink + Send>>>>,
	parent_sinks: Vec<Arc<Mutex<Box<dyn Sink + Send>>>>,
	has_levelless_sinks: bool,
}

impl Logger {
	fn next_uuid() -> u32 {
		let mut next_id = GLOBAL_LOGGER_NEXT_UUID.lock().unwrap();
		let id = *next_id;
		*next_id += 1;

		id
	}

	/// Creates a brand new [`Logger`] instance, with a default level of [`Level::Warning`]
	/// and no associated sinks.
	pub fn new() -> Self {
		Self {
			id: Self::next_uuid(),
			depth: 0,
			level: Level::Warning,
			async_writes: false,
			attributes: attributes::Map::new(),
			sinks: Vec::new(),
			parent_sinks: Vec::new(),
			has_levelless_sinks: false,
		}
	}

	/// Returns `true` if this is a root [`Logger`] instance - i.e. it has no parents.
	pub fn is_root(&self) -> bool {
		return self.depth == 0;
	}

	fn has_sinks(&self) -> bool {
		!self.parent_sinks.is_empty() || !self.sinks.is_empty()
	}

	/// Returns the log [`Level`] for this [`Logger`] instance.
	pub fn level(&self) -> &Level {
		return &self.level;
	}

	/// Sets log [`Level`] for this [`Logger`] instance. Log updates below the
	/// given [`Level`] are ignored.
	pub fn set_level(&mut self, level: Level) -> &mut Self {
		self.level = level;
		if self.has_sinks() {
			self.trace_with("log level updated", [("name", Value::from(level.to_string())), ("new_level", Value::from(level.value()))]);
		}

		self
	}

	/// Enables/disables async mode for this [`Logger`].
	///
	/// When async mode is enabled, log updates return immediately but are queued to
	/// write to the [`Sink`]s associated to the [`Logger`] by a separate worker thread.
	/// Log updates for a given [`Logger`] are guaranteed to write in order.
	pub fn set_async(&mut self, async_writes: bool) -> &mut Self {
		if async_writes == self.async_writes {
			return self;
		}
		self.async_writes = async_writes;

		match self.async_writes {
			true => queue::inc_refcount(),
			false => queue::dec_refcount(),
		};

		if self.has_sinks() {
			self.trace_with(
				if async_writes { "enabled async log updates" } else { "disabled async log updates" },
				[("total_async_loggers", queue::refcount().to_value())],
			);
		}

		self
	}

	/// Adds a new logging [`Sink`] to the [`Logger`] instance.
	///
	/// At least one [`Sink`] is required for logging operations to succeed.
	pub fn add_sink<T: sink::Sink + Send + 'static>(&mut self, sink: T) -> &mut Self {
		// log*() locks sinks, so collect details we want to log about it beforehand
		let name: String = sink.name().into();
		let receives_all_levels = sink.receives_all_levels();

		self.sinks.push(Arc::new(Mutex::new(Box::new(sink))));
		self.has_levelless_sinks |= receives_all_levels;

		self.trace_with(
			"added new log sink",
			[
				("name", Value::from(name)),
				("async", Value::from(self.async_writes)),
				("logs_all_levels", Value::from(receives_all_levels)),
			],
		);

		self
	}

	/// Sets an attribute value for a [`Logger`].
	///
	/// Attributes are key-value pairs of {attribute_name, [`Value`]}, and are attached
	/// to all log operations performed by the [`Logger`]. If the attribute already exists,
	/// it is overwritten.
	///
	/// The provided value must implement [`crate::ToValue`].
	pub fn set<T: ToValue>(&mut self, key: &str, v: T) -> &mut Self {
		self.attributes.insert(key, v.to_value());
		self
	}

	/// Sets an attribute [`Value`] for a [`Logger`].
	pub fn set_value(&mut self, key: &str, val: Value) -> &mut Self {
		self.attributes.insert(key, val);
		self
	}

	fn log_with_two<const X: usize, const Y: usize>(&mut self, level: Level, msg: &str, attrs_1: [(&str, Value); X], attrs_2: [(&str, Value); Y]) -> &mut Self {
		if !self.has_sinks() {
			panic!("tried to log without sinks configured for logger {id}", id = self.id);
		}
		// bail out early on negative log requests
		if !self.has_levelless_sinks && !self.level.covers(&level) {
			return self;
		}

		self.attributes.clear_ephemeral();
		for (k, v) in attrs_1 {
			self.attributes.insert_ephemeral(k, v);
		}
		for (k, v) in attrs_2 {
			self.attributes.insert_ephemeral(k, v);
		}

		let update = sink::LogUpdate::new(Timestamp::now(), level, msg.into());
		let attrs = &self.attributes;

		// if we're about to panic, parse the message before attempting to
		// deliver the log update - and losing ownership.
		let panic_msg: Option<String> = if level == Level::Panic { Some(format::as_panic_string(&update, attrs)) } else { None };

		// TODO: we're locking twice on every sink just to check settings :( improve.
		for asink in self.parent_sinks.iter().chain(self.sinks.iter()) {
			if !self.level.covers(&level) {
				if !asink.lock().unwrap().receives_all_levels() {
					continue;
				}
			}

			let res = match self.async_writes {
				true => {
					queue::log(&asink, &update, &attrs);
					Ok(())
				}
				false => asink.lock().unwrap().log(&update, &attrs),
			};
			if let Err(e) = res {
				panic!("failed to log update {update:?} on sink {name} for logger {id}: {e}", name = asink.lock().unwrap().name(), id = self.id);
			}
		}

		if panic_msg.is_some() {
			// oh no :( log as best we can, then bail out
			self.flush();
			panic!("{}", panic_msg.unwrap());
		}

		self
	}

	/// Logs a message with a given level, and no additional attributes.
	pub fn log(&mut self, level: Level, msg: &str) -> &mut Self {
		self.log_with_two(level, msg, [], [])
	}

	/// Logs a message with a given level and additional attribute [`Value`]s.
	pub fn log_with<const L: usize>(&mut self, level: Level, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with_two(level, msg, attrs, [])
	}

	/// Logs a [`Level::Trace`] message, with no additional attributes.
	pub fn trace(&mut self, msg: &str) -> &mut Self {
		self.trace_with(msg, [])
	}

	/// Logs a [`Level::Trace`] message, with additional attribute [`Value`]s.
	pub fn trace_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with_two(Level::Trace, msg, attrs, [(attributes::KEY_LOGGER_ID, Value::from(self.id))])
	}

	/// Logs a [`Level::Debug`] message, with no additional attributes.
	pub fn debug(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Debug, msg)
	}

	/// Logs a [`Level::Debug`] message, with additional attribute [`Value`]s.
	pub fn debug_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Debug, msg, attrs)
	}

	/// Logs a [`Level::Info`] message, with no additional attributes.
	pub fn info(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Info, msg)
	}

	/// Logs a [`Level::Info`] message, with additional attribute [`Value`]s.
	pub fn info_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Info, msg, attrs)
	}

	/// Logs a [`Level::Warning`] message, with no additional attributes.
	pub fn warn(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Warning, msg)
	}

	/// Logs a [`Level::Warning`] message, with additional attribute [`Value`]s.
	pub fn warn_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Warning, msg, attrs)
	}

	/// Logs a [`Level::Error`] message, with no additional attributes.
	pub fn err(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Error, msg)
	}

	/// Logs a [`Level::Error`] message, with additional attribute [`Value`]s.
	pub fn err_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Error, msg, attrs)
	}

	/// Logs a [`Level::Error`] message for a given [`Error`], with no additional attributes.
	pub fn error<T: Error>(&mut self, error: T, msg: &str) -> &mut Self {
		self.log_with(Level::Error, msg, [(attributes::KEY_ERROR, error.to_string().to_value())])
	}

	/// Logs a [`Level::Error`] message for a given [`Error`], with additional attribute [`Value`]s.
	pub fn error_with<T: Error, const L: usize>(&mut self, error: T, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with_two(Level::Error, msg, attrs, [(attributes::KEY_ERROR, error.to_string().to_value())])
	}

	/// Logs a [`Level::Fatal`] message, with no additional attributes.
	pub fn fatal(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Fatal, msg)
	}

	/// Logs a [`Level::Fatal`] message, with additional attribute [`Value`]s.
	pub fn fatal_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Fatal, msg, attrs)
	}

	/// Logs a [`Level::Panic`] message, with no additional attributes, and panics the current process.
	pub fn panic(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Panic, msg)
	}

	/// Logs a [`Level::Panic`] message, with additional attribute [`Value`]s.
	pub fn panic_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Panic, msg, attrs)
	}

	/// Flushes all pending writes on [`Sink`]s for this [`Logger`].
	///
	/// If async mode is enabled, flushing is deferred via the same queue used to write
	/// log messages. The method will not lock, and return immediately, but actual flushes
	/// will materialize later.
	pub fn flush(&mut self) -> &Self {
		for asink in self.parent_sinks.iter().chain(self.sinks.iter()) {
			// TODO: fix logging.
			//let name = asink.lock().unwrap().as_mut().name();
			let res = match self.async_writes {
				true => {
					queue::flush_sink(&asink);
					Ok(())
				}
				false => asink.lock().unwrap().flush(),
			};
			if let Err(e) = res {
				panic!("failed to flush sink {name} for logger {id}: {e}", name = asink.lock().unwrap().name(), id = self.id);
			}
		}

		self
	}
}

impl Clone for Logger {
	fn clone(&self) -> Self {
		if self.depth >= sink::MAX_LOGDEPTH {
			panic!("cannot clone logger {id} with maximum log depth of {max_depth}", max_depth = sink::MAX_LOGDEPTH, id = self.id);
		}

		let mut parent_sinks: Vec<Arc<Mutex<Box<dyn Sink + Send>>>> = Vec::new();
		for s in &self.sinks {
			parent_sinks.push(s.clone());
		}
		for s in &self.parent_sinks {
			parent_sinks.push(s.clone());
		}

		let mut clone = Self {
			id: Self::next_uuid(),
			depth: self.depth + 1,
			level: self.level,
			// async state is modified via set_async()
			async_writes: false,
			attributes: self.attributes.clone(),
			sinks: Vec::new(),
			parent_sinks: parent_sinks,
			has_levelless_sinks: self.has_levelless_sinks,
		};
		clone.set_async(self.async_writes);

		clone
	}
}

impl Drop for Logger {
	fn drop(&mut self) {
		self.flush();

		// If we're dropping a root logger, make sure any async writes left are processed.
		if self.depth == 0 {
			queue::flush();
		}

		self.set_async(false);
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod basic {
	use crate::sink::MAX_LOGDEPTH;

	use super::*;

	#[test]
	#[should_panic]
	fn panic_no_sinks() {
		let mut log = Logger::new();
		log.set_level(Level::Info).info("this should explode");
	}

	#[test]
	#[should_panic]
	fn panic_log_panics() {
		let mut log = Logger::new();
		log.add_sink(sink::stdout::default()).set_level(Level::Info);

		log.info("this should log fine");
		log.panic("and this should explode");
	}

	#[test]
	fn set_async_before_sinks() {
		let mut log = Logger::new();
		log.set_async(true).add_sink(sink::stdout::default());
		log.info("this should log fine");
	}

	#[test]
	#[should_panic]
	fn max_depth_exceeded() {
		let mut log = Logger::new();
		for _ in 0..MAX_LOGDEPTH + 1 {
			log = log.clone();
		}
	}
}

#[cfg(test)]
mod formatting {
	use super::*;
	use ntime;
	use std::io::{Error, ErrorKind};

	#[test]
	fn sync_formatted_output() {
		struct TestCase<'t> {
			name: &'t str,
			out_format: format::OutputFormat,
			time_format: ntime::Format,
			want: &'t str,
		}

		let test_cases: [TestCase; _] = [
			TestCase {
				name: "default stdout",
				out_format: format::OutputFormat::Compact,
				time_format: ntime::Format::UtcMillisDateTime,
				want: "2026-03-04 15:10:15.000 [INF] root test info
2026-03-04 15:10:16.234 [WRN] root test warn
2026-03-04 15:10:17.468 [INF] first test info number=1
2026-03-04 15:10:18.702 [WRN] first test warn number=1
2026-03-04 15:10:19.936 [DBG] first test debug number=1
2026-03-04 15:10:21.170 [ERR] something failed error=\"oh no\" number=1",
			},
			TestCase {
				name: "stdout with timestamps",
				out_format: format::OutputFormat::Compact,
				time_format: ntime::Format::TimestampNanoseconds,
				want: "1772637015000000000 [INF] root test info
1772637016234000000 [WRN] root test warn
1772637017468000000 [INF] first test info number=1
1772637018702000000 [WRN] first test warn number=1
1772637019936000000 [DBG] first test debug number=1
1772637021170000000 [ERR] something failed error=\"oh no\" number=1",
			},
			// TODO: force color even on terminals not supporting it
			/*
			TestCase {
				name: "default stdout",
				out_format: sink::format::OutputFormat::ColorCompact,
				time_format: time::format::StringFormat::UtcMillisDateTime,
				want: "2026-03-04 15:10:15.000 \u{1b}[32mINF\u{1b}[0m \u{1b}[97mroot test info\u{1b}[0m
2026-03-04 15:10:16.234 \u{1b}[33mWRN\u{1b}[0m \u{1b}[97mroot test warn\u{1b}[0m
2026-03-04 15:10:17.468 \u{1b}[32mINF\u{1b}[0m \u{1b}[97mfirst test info\u{1b}[0m \u{1b}[36mnumber\u{1b}[0m=1\u{1b}[0m
2026-03-04 15:10:18.702 \u{1b}[33mWRN\u{1b}[0m \u{1b}[97mfirst test warn\u{1b}[0m \u{1b}[36mnumber\u{1b}[0m=1\u{1b}[0m
2026-03-04 15:10:19.936 \u{1b}[36mDBG\u{1b}[0m \u{1b}[0mfirst test debug\u{1b}[0m \u{1b}[36mnumber\u{1b}[0m=1\u{1b}[0m
2026-03-04 15:10:21.170 \u{1b}[31mERR\u{1b}[0m \u{1b}[97msomething failed\u{1b}[0m \u{1b}[36merror\u{1b}[0m=\u{1b}[91m\"oh no\"\u{1b}[0m \u{1b}[36mnumber\u{1b}[0m=1\u{1b}[0m",
			},
			*/
			TestCase {
				name: "JSON stdout",
				out_format: format::OutputFormat::Json,
				time_format: ntime::Format::UtcDateTime,
				want: "{\"time\":\"2026-03-04 15:10:15\",\"level\":\"info\",\"message\":\"root test info\"}
{\"time\":\"2026-03-04 15:10:16\",\"level\":\"warning\",\"message\":\"root test warn\"}
{\"time\":\"2026-03-04 15:10:17\",\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"time\":\"2026-03-04 15:10:18\",\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"time\":\"2026-03-04 15:10:19\",\"level\":\"debug\",\"message\":\"first test debug\",\"number\":1}
{\"time\":\"2026-03-04 15:10:21\",\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
			},
			TestCase {
				name: "JSON stdout with timestamps",
				out_format: format::OutputFormat::Json,
				time_format: ntime::Format::TimestampMilliseconds,
				want: "{\"timestamp\":1772637015000,\"level\":\"info\",\"message\":\"root test info\"}
{\"timestamp\":1772637016234,\"level\":\"warning\",\"message\":\"root test warn\"}
{\"timestamp\":1772637017468,\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"timestamp\":1772637018702,\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"timestamp\":1772637019936,\"level\":\"debug\",\"message\":\"first test debug\",\"number\":1}
{\"timestamp\":1772637021170,\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
			},
		];

		for tc in test_cases {
			let got: String;

			{
				let string_sink = sink::string::String::new(sink::string::StringConfig {
					mock_time: true,
					formatter_cfg: format::FormatterConfig {
						format: tc.out_format,
						time_format: tc.time_format,
					},
					..sink::string::StringConfig::default()
				});
				let string_sink_output = string_sink.output();

				let mut log = Logger::new();
				log.add_sink(string_sink).set_level(Level::Info);

				log.info("root test info").warn("root test warn").debug("root test debug");

				let mut nlog = log.clone();
				nlog.set_level(Level::Debug).set("number", 1);
				nlog.info("first test info")
					.warn("first test warn")
					.debug("first test debug")
					.trace("trace log to be ignored")
					.error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");

				got = string_sink_output.lock().unwrap().clone();
			}

			assert_eq!(got, tc.want, "{}", tc.name);
		}
	}

	// TODO: make me deterministic
	/*
	#[test]
	fn async_formatted_output() {
		let string_sink_output: Arc<Mutex<String>>;

		{
			let string_sink = sink::string::String::new(sink::string::StringConfig {
				mock_time: true,
				..sink::string::StringConfig::default()
			});
			string_sink_output = string_sink.output();

			let mut log = Rasant::new();
			log.add_sink(string_sink).set_level(Level::Trace).set_async(true);

			log.info("root test info").warn("root test warn").fatal_with("oh no something_horrible", [("why", "fire!".to_value())]);

			let mut nlog = log.clone();
			nlog.id = 2;
			nlog.set("number", 1);
			nlog.info("first test info").warn("first test warn").error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");
		}

		// collect result only after all loggers are dropped, as we'll race the output otherwise
		let got = string_sink_output.lock().unwrap().clone();
		let want = "2026-03-04 15:10:15.000 [TRA] log level updated name=\"trace\" new_level=0 logger_id=0
2026-03-04 15:10:16.234 [TRA] enabled async log updates total_async_loggers=2 logger_id=0
2026-03-04 15:10:17.468 [INF] root test info
2026-03-04 15:10:18.702 [WRN] root test warn
2026-03-04 15:10:19.936 [FAT] oh no something_horrible why=\"fire!\"
2026-03-04 15:10:21.170 [TRA] enabled async log updates total_async_loggers=2 logger_id=5
2026-03-04 15:10:22.404 [INF] first test info number=1
2026-03-04 15:10:23.638 [WRN] first test warn number=1
2026-03-04 15:10:24.872 [ERR] something failed error=\"oh no\" number=1
2026-03-04 15:10:26.106 [TRA] disabled async log updates number=1 total_async_loggers=1 logger_id=2
2026-03-04 15:10:27.340 [TRA] disabled async log updates total_async_loggers=0 logger_id=0";

		assert_eq!(got, want);
	}
	*/
}
