pub mod value;

use std::fmt;
use std::io::Write;

use crate::attributes::value::{ToValue, Value};

pub const KEY_ERROR: &str = "error";
pub const KEY_LEVEL: &str = "level";
pub const KEY_MESSAGE: &str = "message";
pub const KEY_TIME: &str = "time";
pub const KEY_TIMESTAMP: &str = "timestamp";
pub const KEY_LOGGER_ID: &str = "logger_id";
pub const PRIORITY_KEYS: [&str; 2] = [KEY_MESSAGE, KEY_ERROR];
pub const RESTRICTED_KEYS: [&str; 3] = [KEY_LEVEL, KEY_TIME, KEY_TIMESTAMP];

#[derive(Clone, Debug)]
pub struct Map {
	keys: String,
	values: Vec<Value>,
	key_idxs: Vec<(usize, usize)>,
}

impl Map {
	pub fn new() -> Self {
		Self {
			keys: String::new(),
			values: Vec::new(),
			key_idxs: Vec::new(),
		}
	}

	fn is_key_restricted(&self, key: &str) -> bool {
		RESTRICTED_KEYS.iter().find(|&&pk| pk == key).is_some()
	}

	fn is_key_priority(&self, key: &str) -> bool {
		PRIORITY_KEYS.iter().find(|&&pk| pk == key).is_some()
	}

	fn key_to_idx(&self, key: &str) -> Option<usize> {
		let key_size = key.len();
		let res = self.key_idxs.iter().enumerate().find(|&x| {
			let key_start = x.1.0;
			let key_end = x.1.1;
			if (key_end - key_start + 1) != key_size {
				return false;
			}
			let target = &self.keys[key_start..(key_end + 1)];
			key == target
		});
		match res {
			Some((i, (_, _))) => Some(i),
			None => None,
		}
	}

	fn key_by_idx(&self, i: usize) -> Option<&str> {
		match i < self.key_idxs.len() {
			true => {
				let (key_start, key_end) = self.key_idxs[i];
				Some(&self.keys[key_start..(key_end + 1)])
			}
			false => None,
		}
	}

	pub fn len(&self) -> usize {
		self.key_idxs.len()
	}

	pub fn has(&self, key: &str) -> bool {
		self.key_to_idx(key).is_some()
	}

	pub fn into_iter(&self) -> MapIter<'_> {
		MapIter::new(self)
	}

	pub fn get(&self, key: &str) -> Option<&Value> {
		match self.key_to_idx(key) {
			Some(i) => Some(&self.values[i]),
			None => None,
		}
	}

	pub fn insert_val(&mut self, key: &str, val: Value) {
		if key.len() == 0 {
			panic!("empty log attribute key {{\"\" -> {val}}}");
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {val}}}");
		}
		if self.is_key_restricted(key) {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {val}}}");
		}

		match self.key_to_idx(key) {
			Some(i) => {
				// overwrite existing key
				self.values[i] = val;
			}
			None => {
				// new key
				let key_len = key.len();
				if self.is_key_priority(key) {
					// insert new key first
					for (key_start, key_end) in self.key_idxs.iter_mut() {
						*key_start += key_len;
						*key_end += key_len;
					}
					self.keys.insert_str(0, key);
					self.key_idxs.insert(0, (0, key_len - 1));
					self.values.insert(0, val);
				} else {
					// insert new key last
					let key_start = self.keys.len();
					let key_end = key_start + key_len - 1;
					self.key_idxs.push((key_start, key_end));
					self.keys.push_str(key);
					self.values.push(val);
				};
			}
		};
	}

	pub fn insert<T: ToValue>(&mut self, key: &str, raw: T) {
		self.insert_val(key, raw.to_value());
	}
}

pub struct MapIter<'s> {
	map: &'s Map,
	idx: usize,
}

impl<'i> MapIter<'i> {
	pub fn new(map: &'i Map) -> Self {
		Self { map: map, idx: 0 }
	}
}

impl<'i> Iterator for MapIter<'i> {
	// {key: value}
	type Item = (&'i str, &'i Value);

	fn next(&mut self) -> Option<Self::Item> {
		match self.map.key_by_idx(self.idx) {
			Some(key) => {
				let res = (key, &self.map.values[self.idx]);
				self.idx += 1;
				Some(res)
			}
			None => None,
		}
	}
}

// TODO: implement proper glue between io::Write and fmt::Write
impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut out = Vec::new();

		let mut first = true;
		for (key, val) in self.into_iter() {
			match write!(&mut out, "{spacer}{key}=", spacer = if first { "" } else { " " }) {
				Ok(_) => (),
				Err(e) => panic!("failed to serialize key for attributes string: {e}"),
			}
			match val.write_quoted(&mut out) {
				Ok(_) => (),
				Err(e) => panic!("failed to serialize value for attributes string: {e}"),
			}
			first = false;
		}

		let s = match String::from_utf8(out) {
			Ok(s) => s,
			Err(e) => panic!("failed to convert attributes to UTF8: {e}"),
		};
		write!(f, "{}", s)
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn indexed_keys_order() {
		let mut map = Map::new();

		map.insert("key_a", 123);
		map.insert("key_b", 456);
		map.insert("key_c", 789);
		map.insert("key_b", "overwrites should not change key order");
		map.insert("error", "priority keys should go first");

		assert_eq!(map.len(), 4);
		assert_eq!(map.key_to_idx("error"), Some(0));
		assert_eq!(map.key_to_idx("key_a"), Some(1));
		assert_eq!(map.key_to_idx("key_b"), Some(2));
		assert_eq!(map.key_to_idx("key_c"), Some(3));
		assert_eq!(map.key_to_idx("bad_key"), None);
	}

	#[test]
	fn basic_operations() {
		let mut map = Map::new();

		assert_eq!(map.len(), 0);

		map.insert("c", -5678);
		map.insert("d", 9012.3456);
		map.insert("b", 1234);
		// overwrite existing key
		map.insert("d", 7890.1234);
		map.insert("error", "first!");
		map.insert_val("e", Value::Size(77889900));
		map.insert("a", "lalala");

		assert_eq!(map.len(), 6);
		assert_eq!(map.to_string(), "error=\"first!\" c=-5678 d=7890.1234 b=1234 e=0x4a4816c a=\"lalala\"");
	}

	#[test]
	#[should_panic]
	fn insert_empty_key() {
		let mut map = Map::new();
		map.insert("", "oh no");
	}

	#[test]
	#[should_panic]
	fn insert_invalid_key() {
		let mut map = Map::new();
		map.insert("no\twhitespace\tin\tkeys", "please!");
	}

	#[test]
	#[should_panic]
	fn insert_restricted_key() {
		let mut map = Map::new();
		map.insert("level", 55555);
	}
}
