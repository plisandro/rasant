pub mod value;

use std::fmt;

use crate::attributes::value::Value;
use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP};

macro_rules! check_match {
	($a:ident, $( $b:ident ),*) => {
	    {
	        $( if $a == $b { return true; } )*
			false
		}
	};
}

fn is_key_priority(key: &str) -> bool {
	check_match!(key, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_ERROR)
}

fn is_key_restricted(key: &str) -> bool {
	check_match!(key, ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP)
}

#[derive(Clone, Debug)]
struct KvStore {
	keys: String,
	values: Vec<Value>,
	key_idxs: Vec<(usize, usize)>,
}

impl KvStore {
	fn new() -> Self {
		Self {
			keys: String::new(),
			values: Vec::new(),
			key_idxs: Vec::new(),
		}
	}

	fn clear(&mut self) {
		self.keys.clear();
		self.values.clear();
		self.key_idxs.clear();
	}

	fn key_to_idx(&self, key: &str) -> Option<usize> {
		let key_size = key.len();
		let res = self.key_idxs.iter().enumerate().find(|(_, (key_start, key_end))| {
			let (key_start, key_end) = (*key_start, *key_end);
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

	fn value_by_idx(&self, i: usize) -> &Value {
		&self.values[i]
	}

	fn len(&self) -> usize {
		self.key_idxs.len()
	}

	fn has(&self, key: &str) -> bool {
		self.key_to_idx(key).is_some()
	}

	fn get(&self, key: &str) -> Option<&Value> {
		match self.key_to_idx(key) {
			Some(i) => Some(&self.values[i]),
			None => None,
		}
	}

	fn set(&mut self, key: &str, val: Value) {
		if key.len() == 0 {
			panic!("empty log attribute key {{\"\" -> {val}}}");
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {val}}}");
		}
		if is_key_restricted(key) {
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
				if is_key_priority(key) {
					// insert new priority key first
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
		}
	}
}

#[derive(Clone, Debug)]
pub struct Map {
	main: KvStore,
	ephemeral_new: KvStore,
	ephemeral_priority: KvStore,
	ephemeral_overlap: KvStore,
}

impl Map {
	pub fn new() -> Self {
		Self {
			main: KvStore::new(),
			ephemeral_new: KvStore::new(),
			ephemeral_priority: KvStore::new(),
			ephemeral_overlap: KvStore::new(),
		}
	}

	pub fn into_iter(&self) -> MapIter<'_> {
		MapIter::new(self)
	}

	pub fn len(&self) -> usize {
		self.main.len() + self.ephemeral_new.len() + self.ephemeral_priority.len()
	}

	pub fn has(&self, key: &str) -> bool {
		self.main.has(key) || self.ephemeral_new.has(key) || self.ephemeral_priority.has(key)
	}

	pub fn get(&self, key: &str) -> Option<&Value> {
		if let Some(val) = self.ephemeral_new.get(key) {
			return Some(val);
		}
		if let Some(val) = self.ephemeral_priority.get(key) {
			return Some(val);
		}
		self.main.get(key)
	}

	pub fn insert(&mut self, key: &str, val: Value) {
		_ = self.main.set(key, val);
	}

	pub fn clear_ephemeral(&mut self) {
		self.ephemeral_new.clear();
		self.ephemeral_priority.clear();
		self.ephemeral_overlap.clear();
	}

	pub fn insert_ephemeral(&mut self, key: &str, val: Value) {
		match self.main.has(key) {
			false => match is_key_priority(key) {
				true => self.ephemeral_priority.set(key, val),
				false => self.ephemeral_new.set(key, val),
			},
			true => self.ephemeral_overlap.set(key, val),
		}
	}
}

pub struct MapIter<'s> {
	map: &'s Map,
	main_idx: usize,
	ephemeral_new_idx: usize,
	ephemeral_priority_idx: usize,
}

impl<'i> MapIter<'i> {
	pub fn new(map: &'i Map) -> Self {
		Self {
			map: map,
			main_idx: 0,
			ephemeral_new_idx: 0,
			ephemeral_priority_idx: 0,
		}
	}
}

impl<'i> Iterator for MapIter<'i> {
	// {key: value}
	type Item = (&'i str, &'i Value);

	fn next(&mut self) -> Option<Self::Item> {
		// iterate over priority ephemeral KVs
		match self.map.ephemeral_priority.key_by_idx(self.ephemeral_priority_idx) {
			None => (),
			Some(key) => {
				let val = self.map.ephemeral_priority.value_by_idx(self.ephemeral_priority_idx);
				self.ephemeral_priority_idx += 1;
				return Some((key, val));
			}
		}

		// iterate over main KV and ephemeral value overlaps
		match self.map.main.key_by_idx(self.main_idx) {
			None => (),
			Some(key) => {
				let val = match self.map.ephemeral_overlap.get(key) {
					Some(v) => v,
					None => self.map.main.value_by_idx(self.main_idx),
				};

				self.main_idx += 1;
				return Some((key, val));
			}
		}

		// iterate over the rest of ephemeral KVs
		match self.map.ephemeral_new.key_by_idx(self.ephemeral_new_idx) {
			None => None,
			Some(key) => {
				let val = self.map.ephemeral_new.value_by_idx(self.ephemeral_new_idx);
				self.ephemeral_new_idx += 1;
				Some((key, val))
			}
		}
	}
}

impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut first: bool = true;
		for (key, val) in self.into_iter() {
			write!(f, "{spacer}{key}={val}", spacer = if first { "" } else { " " })?;
			first = false;
		}
		Ok(())
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod kv_store_tests {
	use super::*;
	use crate::attributes::value::ToValue;

	#[test]
	fn indexed_keys_order() {
		let mut kv = KvStore::new();

		kv.set("key_a", 123.to_value());
		kv.set("key_b", 456.to_value());
		kv.set("key_c", 789.to_value());
		kv.set("key_b", "overwrites should not change key order".to_value());
		kv.set("error", "priority keys should go first".to_value());

		assert_eq!(kv.len(), 4);
		assert_eq!(kv.key_to_idx("error"), Some(0));
		assert_eq!(kv.key_to_idx("key_a"), Some(1));
		assert_eq!(kv.key_to_idx("key_b"), Some(2));
		assert_eq!(kv.key_to_idx("key_c"), Some(3));
		assert_eq!(kv.key_to_idx("bad_key"), None);
	}

	#[test]
	fn basic_operations() {
		let mut kv = KvStore::new();

		assert_eq!(kv.len(), 0);

		kv.set("c", (-5678).to_value());
		kv.set("d", (9012.3456).to_value());
		kv.set("b", (1234).to_value());
		// overwrite existing key
		kv.set("d", (7890.1234).to_value());
		kv.set("error", "first!".to_value());
		kv.set("e", Value::Size(77889900));
		kv.set("a", "lalala".to_value());

		assert_eq!(kv.len(), 6);
	}

	#[test]
	#[should_panic]
	fn insert_empty_key() {
		KvStore::new().set("", "oh no".to_value());
	}

	#[test]
	#[should_panic]
	fn insert_invalid_key() {
		KvStore::new().set("no\twhitespace\tin\tkeys", "please!".to_value());
	}

	#[test]
	#[should_panic]
	fn insert_restricted_key() {
		KvStore::new().set("level", 55555i32.to_value());
	}
}

#[cfg(test)]
mod map_tests {
	use super::*;
	use crate::attributes::value::ToValue;

	#[test]
	fn indexed_keys_order() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert("key_b", 456.to_value());
		attr.insert("key_c", 789.to_value());
		attr.insert("key_b", "overwrites should not change key order".to_value());
		attr.insert("error", "priority keys should go first".to_value());

		assert_eq!(attr.len(), 4);
		assert_eq!(
			attr.to_string(),
			"error=\"priority keys should go first\" key_a=123 key_b=\"overwrites should not change key order\" key_c=789"
		);
	}

	#[test]
	fn ephemeral_attributes() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert("key_b", 456.to_value());
		attr.insert("key_c", 789.to_value());
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrites should not change key order".to_value());
		attr.insert_ephemeral("key_d", "new key".to_value());
		attr.insert_ephemeral("error", "new error".to_value());

		assert_eq!(attr.len(), 5);
		assert_eq!(attr.main.len(), 4);
		assert_eq!(attr.ephemeral_new.len(), 1);
		assert_eq!(attr.ephemeral_priority.len(), 0);
		assert_eq!(attr.ephemeral_overlap.len(), 2);
		assert_eq!(
			attr.to_string(),
			"error=\"new error\" key_a=123 key_b=\"overwrites should not change key order\" key_c=789 key_d=\"new key\"",
		);

		attr.clear_ephemeral();

		assert_eq!(attr.len(), 4);
		assert_eq!(attr.main.len(), 4);
		assert_eq!(attr.ephemeral_new.len(), 0);
		assert_eq!(attr.ephemeral_priority.len(), 0);
		assert_eq!(attr.ephemeral_overlap.len(), 0);
		assert_eq!(attr.to_string(), "error=\"first error\" key_a=123 key_b=456 key_c=789",);
	}

	#[test]
	fn ephemeral_new_priority_keys() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert("key_b", 456.to_value());
		attr.insert("key_c", 789.to_value());

		attr.insert_ephemeral("error", "oh no!".to_value());
		attr.insert_ephemeral("key_d", "new key".to_value());

		assert_eq!(attr.len(), 5);
		assert_eq!(attr.main.len(), 3);
		assert_eq!(attr.ephemeral_new.len(), 1);
		assert_eq!(attr.ephemeral_priority.len(), 1);
		assert_eq!(attr.ephemeral_overlap.len(), 0);
		assert_eq!(attr.to_string(), "error=\"oh no!\" key_a=123 key_b=456 key_c=789 key_d=\"new key\"",);
	}
}
