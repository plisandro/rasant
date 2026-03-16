pub mod attributes;
mod console;
pub mod level;
pub mod sink;
pub mod sys;
pub mod time;

use level::Level;
use std::error::Error;
use std::iter::Chain;
use std::slice::Iter;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::attributes::KEY_ERROR;
use crate::attributes::value::{ToValue, Value};
use crate::sink::Sink;
use crate::sink::format;
use crate::sink::wrapper::AsyncSink;

pub struct Slog<'s> {
	depth: sink::LogDepth,
	level: level::Level,
	async_writes: bool,
	attributes: attributes::Map,
	sinks: Vec<Arc<Mutex<Box<dyn Sink + 's>>>>,
	parent_sinks: Vec<Arc<Mutex<Box<dyn Sink + 's>>>>,
	has_levelless_sinks: bool,
}

impl<'s> Slog<'s> {
	pub fn new() -> Self {
		Self {
			depth: 0,
			level: Level::Warning,
			async_writes: false,
			attributes: attributes::Map::new(),
			sinks: Vec::new(),
			parent_sinks: Vec::new(),
			has_levelless_sinks: false,
		}
	}

	pub fn is_root(&self) -> bool {
		return self.depth == 0;
	}

	pub fn clone(&self) -> Self {
		if self.depth >= sink::LOGDEPTH_MAX {
			panic!("maximum log depth of {} exceeded", sink::LOGDEPTH_MAX);
		}

		let mut parent_sinks: Vec<Arc<Mutex<Box<dyn Sink>>>> = Vec::new();
		for s in &self.sinks {
			parent_sinks.push(s.clone());
		}
		for s in &self.parent_sinks {
			parent_sinks.push(s.clone());
		}

		Self {
			depth: self.depth + 1,
			level: self.level,
			async_writes: self.async_writes,
			attributes: self.attributes.clone(),
			sinks: Vec::new(),
			parent_sinks: parent_sinks,
			has_levelless_sinks: self.has_levelless_sinks,
		}
	}

	pub fn level(&self) -> &level::Level {
		return &self.level;
	}

	pub fn set_level(&mut self, level: level::Level) -> &mut Self {
		self.level = level;
		self.log_with(
			level::Level::Info,
			"log level updated",
			[("name", Value::from(level.to_string())), ("new_level", Value::from(level.value()))],
		);

		self
	}

	pub fn set_async(&mut self) -> &mut Self {
		if !self.async_writes && !self.sinks.is_empty() {
			panic!("cannot enable async updates after log sinks have been configured");
		}

		todo!("not yet supported");

		self.async_writes = true;
		self
	}

	pub fn set_sync(&mut self) -> &mut Self {
		if self.async_writes && !self.sinks.is_empty() {
			panic!("cannot enable sync updates after log sinks have been configured");
		}

		self.async_writes = false;
		self
	}

	// TODO: fix lifetime.
	pub fn add_sink<T: sink::Sink + 's>(&mut self, sink: T) -> &mut Self {
		// log*() locks sinks, so collect details we want to log about it beforehand
		let name: String = sink.name().into();
		let receives_all_levels = sink.receives_all_levels();

		let bsink: Box<dyn Sink>;
		if self.async_writes {
			bsink = Box::new(AsyncSink::new(sink));
		} else {
			bsink = Box::new(sink);
		}
		self.sinks.push(Arc::new(Mutex::new(bsink)));
		self.has_levelless_sinks |= receives_all_levels;

		self.log_with(
			level::Level::Debug,
			"added new log sink",
			[
				("name", Value::from(name)),
				("id", Value::from(thread::current().id())),
				("async", Value::from(self.async_writes)),
				("logs_all_levels", Value::from(receives_all_levels)),
			],
		);

		self
	}

	pub fn set<T: ToValue>(&mut self, key: &str, v: T) -> &mut Self {
		self.attributes.insert(key, v);
		self
	}

	fn log_with_two<const X: usize, const Y: usize>(&mut self, level: level::Level, msg: &str, attrs_1: [(&str, Value); X], attrs_2: [(&str, Value); Y]) -> &mut Self {
		if self.parent_sinks.is_empty() && self.sinks.is_empty() {
			panic!("tried to log without sinks configured");
		}
		// bail out early on negative log requests
		if !self.has_levelless_sinks && !self.level.covers(&level) {
			return self;
		}

		let mut nattrs = self.attributes.clone();
		for a in attrs_1 {
			nattrs.insert_val(a.0, a.1);
		}
		for a in attrs_2 {
			nattrs.insert_val(a.0, a.1);
		}

		let update = sink::LogUpdate::new(time::Timestamp::now(), level, self.depth, msg.into(), nattrs);

		// if we're about to panic, parse the message before attempting to
		// deliver the log update - and losing ownership.
		let panic_msg: Option<String> = if level != Level::Panic {
			None
		} else {
			let formatter = sink::format::Formatter::new(format::FormatterConfig::default());
			Some(formatter.as_string(&update))
		};

		for sink in self.parent_sinks.iter().chain(self.sinks.iter()) {
			let mut sink = sink.lock().unwrap();
			if sink.receives_all_levels() || self.level.covers(&level) {
				if let Err(e) = sink.log(&update) {
					panic!("failed to log update {update:?} on sink {name}: {e}", name = sink.name());
				}
			}
		}

		if panic_msg.is_some() {
			// oh no :( log as best we can, then bail out
			self.flush();
			panic!("{}", panic_msg.unwrap());
		}

		self
	}

	pub fn log(&mut self, level: level::Level, msg: &str) -> &mut Self {
		self.log_with_two(level, msg, [], [])
	}

	pub fn log_with<const L: usize>(&mut self, level: level::Level, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with_two(level, msg, attrs, [])
	}

	pub fn trace(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Trace, msg)
	}

	pub fn trace_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Trace, msg, attrs)
	}

	pub fn debug(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Debug, msg)
	}

	pub fn debug_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Debug, msg, attrs)
	}

	pub fn info(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Info, msg)
	}

	pub fn info_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Info, msg, attrs)
	}

	pub fn warn(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Warning, msg)
	}

	pub fn warn_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Warning, msg, attrs)
	}

	pub fn err(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Error, msg)
	}

	pub fn err_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Error, msg, attrs)
	}

	pub fn error<T: Error>(&mut self, error: T, msg: &str) -> &mut Self {
		self.log_with(Level::Error, msg, [(KEY_ERROR, error.to_string().to_value())])
	}

	pub fn error_with<T: Error + ToValue, const L: usize>(&mut self, msg: &str, error: T, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with_two(Level::Error, msg, attrs, [(KEY_ERROR, error.to_value())])
	}

	pub fn fatal(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Fatal, msg)
	}

	pub fn fatal_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Fatal, msg, attrs)
	}

	pub fn panic(&mut self, msg: &str) -> &mut Self {
		self.log(Level::Panic, msg)
	}

	pub fn panic_with<const L: usize>(&mut self, msg: &str, attrs: [(&str, Value); L]) -> &mut Self {
		self.log_with(Level::Panic, msg, attrs)
	}

	pub fn flush(&self) -> &Self {
		for sink in self.parent_sinks.iter().chain(self.sinks.iter()) {
			let mut sink = sink.lock().unwrap();
			if let Err(e) = sink.flush() {
				panic!("failed to flush sink {name}: {e}", name = sink.name());
			}
		}

		self
	}

	fn drop(&self) {
		self.flush();
	}
}
