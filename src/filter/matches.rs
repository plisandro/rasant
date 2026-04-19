//! Log update matchers [`filter`] module.
//!
//! Provides log update filters matching substrings in the message body,
//! presence of attributes keys, and attribute value contents.
//!
//! Note that matchers are comparatively expensive to run (against the
//! rest of Rasant) as they operate on every single meesage, attribute key
//! and attribute value of every log update received.
//!

use std::string;

use crate::attributes;
use crate::filter;
use crate::sink;

/// Configuration struct for a log message [`filter`].
pub struct MessageConfig<'s, const A: usize, const B: usize> {
	/// A list of substrings which must be present in the log message.
	pub has: [&'s str; A],
	/// A list of substrings which must **not** be present in the log message.
	pub has_not: [&'s str; B],
	/// Whether to expect all has/has-not log message substrings, or any of them.
	pub match_all: bool,
}

/// A log message [filter][`filter::Filter`], which filters log updates based
/// on message contents.
pub struct Message {
	name: string::String,
	has: Vec<String>,
	has_not: Vec<String>,
	match_all: bool,
}

impl Message {
	/// Initializes a new message log [`filter`], from a given [`MessageConfig`].
	pub fn new<const A: usize, const B: usize>(conf: MessageConfig<A, B>) -> Self {
		Self {
			name: format!(
				"message matcher ({how} has:{has}, has_not:{has_not}",
				how = if conf.match_all { "all" } else { "any" },
				has = conf.has.len(),
				has_not = conf.has_not.len()
			),
			match_all: conf.match_all,
			has: conf.has.iter().map(|x: &&str| x.to_string()).collect(),
			has_not: conf.has_not.iter().map(|x: &&str| x.to_string()).collect(),
		}
	}
}

impl filter::Filter for Message {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, update: &sink::LogUpdate, _: &attributes::Map) -> bool {
		match self.match_all {
			true => {
				if !self.has.iter().all(|x| update.msg.contains((*x).as_str())) {
					return false;
				}
				if !self.has_not.iter().all(|x| !update.msg.contains((*x).as_str())) {
					return false;
				}
			}
			false => {
				if !self.has.is_empty() && !self.has.iter().any(|x| update.msg.contains((*x).as_str())) {
					return false;
				}
				if !self.has_not.is_empty() && !self.has_not.iter().any(|x| !update.msg.contains((*x).as_str())) {
					return false;
				}
			}
		}

		true
	}
}

/// Configuration struct for an attribute key [`filter`].
pub struct AttributeKeyConfig<'s, const A: usize, const B: usize> {
	/// A list of attribute keys which must be present in the log update.
	pub has: [&'s str; A],
	/// A list of attribute keys which must **not** be present in the log update.
	pub has_not: [&'s str; B],
	/// Whether to expect all has/has-not attribute keys, or any of them.
	pub match_all: bool,
}

/// An attribute key [filter][`filter::Filter`], which filters log updates based
/// on the presence of attribute key(s).
pub struct AttributeKey {
	name: string::String,
	has: Vec<String>,
	has_not: Vec<String>,
	match_all: bool,
}

impl AttributeKey {
	/// Initializes a new message log [`filter`], from a given [`MessageConfig`].
	pub fn new<const A: usize, const B: usize>(conf: AttributeKeyConfig<A, B>) -> Self {
		Self {
			name: format!(
				"attribute key matcher ({how} has:{has}, has_not:{has_not}",
				how = if conf.match_all { "all" } else { "any" },
				has = conf.has.len(),
				has_not = conf.has_not.len()
			),
			match_all: conf.match_all,
			has: conf.has.iter().map(|x: &&str| x.to_string()).collect(),
			has_not: conf.has_not.iter().map(|x: &&str| x.to_string()).collect(),
		}
	}
}

impl filter::Filter for AttributeKey {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, attrs: &attributes::Map) -> bool {
		let mut has_matches: usize = 0;
		let mut has_not_matches: usize = 0;

		for key in attrs.key_iter() {
			has_matches += self.has.iter().filter(|x| key == *x).count();
			has_not_matches += self.has_not.iter().filter(|x| key == *x).count();
		}
		has_not_matches = self.has_not.len() - has_not_matches;

		match self.match_all {
			true => has_matches == self.has.len() && has_not_matches == self.has_not.len(),
			false => {
				let mut res = true;
				if !self.has.is_empty() {
					res &= has_matches > 0;
				}
				if !self.has_not.is_empty() {
					res &= has_not_matches > 0;
				}

				res
			}
		}
	}
}

/// Configuration struct for an attribute value [`filter`].
pub struct AttributeValueConfig<'s, const A: usize, const B: usize> {
	/// Attribute key to match on (if present).
	pub key: &'s str,
	/// A list of substrings which must be present in the attribute value.
	pub has: [&'s str; A],
	/// A list of substrings which must **not** be present in the attribute value.
	pub has_not: [&'s str; B],
	/// Whether to expect all has/has-not attribute value substrings, or any of them.
	pub match_all: bool,
}

/// An attribute value [filter][`filter::Filter`], which filters log updates based
/// on the presence of am attribute, and its value conntents.
pub struct AttributeValue {
	name: string::String,
	key: String,
	has: Vec<String>,
	has_not: Vec<String>,
	match_all: bool,
}

impl AttributeValue {
	/// Initializes a new message log [`filter`], from a given [`MessageConfig`].
	pub fn new<const A: usize, const B: usize>(conf: AttributeValueConfig<A, B>) -> Self {
		Self {
			name: format!(
				"attribute value matcher (on \"{key}\" {how} has:{has}, has_not:{has_not}",
				key = conf.key,
				how = if conf.match_all { "all" } else { "any" },
				has = conf.has.len(),
				has_not = conf.has_not.len()
			),
			key: conf.key.into(),
			match_all: conf.match_all,
			has: conf.has.iter().map(|x: &&str| x.to_string()).collect(),
			has_not: conf.has_not.iter().map(|x: &&str| x.to_string()).collect(),
		}
	}
}

impl filter::Filter for AttributeValue {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, attrs: &attributes::Map) -> bool {
		let val = match attrs.get(self.key.as_str()) {
			Some(v) => (*v).to_string(),
			None => return false,
		};

		match self.match_all {
			true => {
				if !self.has.iter().all(|x| val.contains((*x).as_str())) {
					return false;
				}
				if !self.has_not.iter().all(|x| !val.contains((*x).as_str())) {
					return false;
				}
			}
			false => {
				if !self.has.is_empty() && !self.has.iter().any(|x| val.contains((*x).as_str())) {
					return false;
				}
				if !self.has_not.is_empty() && !self.has_not.iter().any(|x| !val.contains((*x).as_str())) {
					return false;
				}
			}
		}

		true
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;
	use crate::{Level, ToValue};

	#[test]
	fn message() {
		fn run(mut filter: Message, want: bool) {
			let args = attributes::Map::new();
			let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "this is a test log".into());
			assert_eq!(filter.pass(&update, &args), want);
		}

		run(
			Message::new(MessageConfig {
				has: [],
				has_not: [],
				match_all: false,
			}),
			true,
		);

		// Message filter with has.
		run(
			Message::new(MessageConfig {
				has: ["this is a"],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["thIS IS a"],
				has_not: [],
				match_all: false,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: ["test", "log"],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["test", "log"],
				has_not: [],
				match_all: true,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["tEsT", "log"],
				has_not: [],
				match_all: true,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: ["tEsT", "log"],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["tEsT", "lXg"],
				has_not: [],
				match_all: true,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: ["tEsT", "lXg"],
				has_not: [],
				match_all: false,
			}),
			false,
		);

		// Message filter with has_not.
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["this is a"],
				match_all: false,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["thIS IS a"],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["test", "log"],
				match_all: false,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["test", "log"],
				match_all: true,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["tEsT", "log"],
				match_all: true,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["tEsT", "log"],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["tEsT", "lXg"],
				match_all: true,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: [],
				has_not: ["tEsT", "lXg"],
				match_all: false,
			}),
			true,
		);

		// Message filter with has + has_not.
		run(
			Message::new(MessageConfig {
				has: ["this", "is"],
				has_not: ["test", "log"],
				match_all: false,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: ["this", "is"],
				has_not: ["test", "log"],
				match_all: true,
			}),
			false,
		);
		run(
			Message::new(MessageConfig {
				has: ["this", "is"],
				has_not: ["tEsT", "lXg"],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["this", "is"],
				has_not: ["tEsT", "lXg"],
				match_all: true,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["tHiS", "is"],
				has_not: ["tEsT", "log"],
				match_all: false,
			}),
			true,
		);
		run(
			Message::new(MessageConfig {
				has: ["tHiS", "is"],
				has_not: ["tEsT", "log"],
				match_all: true,
			}),
			false,
		);
	}

	#[test]
	fn attribute_keys() {
		fn run(mut filter: AttributeKey, want: bool) {
			let mut args = attributes::Map::new();
			args.insert("a_string", "hello there!".to_value());
			args.insert("an_int", 12345.to_value());
			args.insert("a_float", (6789.0123 as f32).to_value());

			let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "unused update :(".into());
			assert_eq!(filter.pass(&update, &args), want);
		}

		run(
			AttributeKey::new(AttributeKeyConfig {
				has: [],
				has_not: [],
				match_all: false,
			}),
			true,
		);

		// Attribute key filter with has.
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["a_bool"],
				has_not: [],
				match_all: false,
			}),
			false,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["a_string", "a_bool"],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["a_string", "a_bool"],
				has_not: [],
				match_all: true,
			}),
			false,
		);

		// Attribute key filter with has_not.
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: [],
				has_not: ["a_float"],
				match_all: false,
			}),
			false,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: [],
				has_not: ["a_string", "a_bool"],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: [],
				has_not: ["a_string", "a_bool"],
				match_all: true,
			}),
			false,
		);

		// Attribute key filter with has + has_not.
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["an_int", "a_float"],
				has_not: ["a_bool", "an_usize"],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["an_int", "a_float"],
				has_not: ["a_bool", "an_usize"],
				match_all: true,
			}),
			true,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["an_int", "a_bool"],
				has_not: ["a_float", "an_usize"],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeKey::new(AttributeKeyConfig {
				has: ["an_int", "a_bool"],
				has_not: ["a_float", "an_usize"],
				match_all: true,
			}),
			false,
		);
	}

	#[test]
	fn attribute_values() {
		fn run(mut filter: AttributeValue, want: bool) {
			let mut args = attributes::Map::new();
			args.insert("a_string", "hello there!".to_value());
			args.insert("an_int", 12345.to_value());
			args.insert("a_float", (6789.0123 as f32).to_value());

			let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "unused update :(".into());
			assert_eq!(filter.pass(&update, &args), want);
		}

		run(
			AttributeValue::new(AttributeValueConfig {
				key: "",
				has: [],
				has_not: [],
				match_all: false,
			}),
			false,
		);
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "a_string",
				has: [],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "wrong",
				has: [],
				has_not: [],
				match_all: false,
			}),
			false,
		);

		// Attribute key filter with has.
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "an_int",
				has: ["1234", "wrong"],
				has_not: [],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "an_int",
				has: ["1234", "wrong"],
				has_not: [],
				match_all: true,
			}),
			false,
		);

		// Attribute key filter with has_not.
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "a_string",
				has: [],
				has_not: ["hello", "tHeRe"],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "a_string",
				has: [],
				has_not: ["hello", "tHeRe"],
				match_all: true,
			}),
			false,
		);

		// Attribute key filter with has + has_not.
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "a_string",
				has: ["hello", "tHeRe"],
				has_not: ["there!", "123456"],
				match_all: false,
			}),
			true,
		);
		run(
			AttributeValue::new(AttributeValueConfig {
				key: "a_string",
				has: ["hello", "tHeRe"],
				has_not: ["there!", "123456"],
				match_all: true,
			}),
			false,
		);
	}
}
