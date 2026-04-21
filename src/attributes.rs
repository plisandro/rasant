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
	value_idxs: Vec<(usize, usize)>,
}

impl KvStore {
	fn new() -> Self {
		Self {
			keys: String::new(),
			values: Vec::new(),
			key_idxs: Vec::new(),
			value_idxs: Vec::new(),
		}
	}

	fn clear(&mut self) {
		self.keys.clear();
		self.values.clear();
		self.key_idxs.clear();
		self.value_idxs.clear();
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

	fn values_by_idx(&self, i: usize) -> &[Value] {
		let (start, end) = self.value_idxs[i];
		&self.values[start..end + 1]
	}

	fn len(&self) -> usize {
		self.key_idxs.len()
	}

	fn store_size(&self) -> usize {
		self.values.len()
	}

	fn has(&self, key: &str) -> bool {
		self.key_to_idx(key).is_some()
	}

	fn get(&self, key: &str) -> Option<&[Value]> {
		match self.key_to_idx(key) {
			Some(i) => Some(self.values_by_idx(i)),
			None => None,
		}
	}

	fn set<const N: usize>(&mut self, key: &str, vals: &[Value; N]) {
		if key.len() == 0 {
			panic!("empty log attribute key {{\"\" -> {vals:?}}}");
		}
		if vals.len() == 0 {
			panic!("empty log attribute values {{\"{key}\" -> {vals:?}}}");
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {vals:?}}}");
		}
		if is_key_restricted(key) {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {vals:?}}}");
		}

		match self.key_to_idx(key) {
			Some(i) => {
				// overwrite existing key
				let (pre_start, pre_end) = self.value_idxs[i];
				let pre_size = pre_end - pre_start + 1;
				match vals.len() == pre_size {
					true => {
						// yay, new values fit in the existing slot
						// TODO: is there any way to copy instead?
						self.values[pre_start..pre_end + 1].clone_from_slice(vals);
					}
					false => {
						// we need to resize :'(
						for (start, end) in &mut self.value_idxs {
							if *start >= pre_size && *start > pre_start {
								*start -= pre_size;
								*end -= pre_size;
							}
						}

						self.values.drain(pre_start..pre_end + 1);
						let start_idx = self.values.len();
						let end_idx = start_idx + vals.len() - 1;
						self.values.extend_from_slice(vals);
						self.value_idxs[i] = (start_idx, end_idx);
					}
				}
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

					let start_idx = self.values.len();
					let end_idx = start_idx + vals.len() - 1;
					self.values.extend_from_slice(vals);
					self.value_idxs.insert(0, (start_idx, end_idx));
				} else {
					// insert new key last
					let key_start = self.keys.len();
					let key_end = key_start + key_len - 1;
					self.key_idxs.push((key_start, key_end));
					self.keys.push_str(key);

					let start_idx = self.values.len();
					let end_idx = start_idx + vals.len() - 1;
					self.values.extend_from_slice(vals);
					self.value_idxs.push((start_idx, end_idx));
				}
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

	pub fn iter(&self) -> MapIter<'_> {
		MapIter::new(self)
	}

	pub fn key_iter(&self) -> MapKeyIter<'_> {
		MapKeyIter::new(self)
	}

	pub fn len(&self) -> usize {
		self.main.len() + self.ephemeral_new.len() + self.ephemeral_priority.len()
	}

	pub fn has(&self, key: &str) -> bool {
		self.main.has(key) || self.ephemeral_new.has(key) || self.ephemeral_priority.has(key)
	}

	pub fn get(&self, key: &str) -> Option<&[Value]> {
		if let Some(vals) = self.ephemeral_new.get(key) {
			return Some(vals);
		}
		if let Some(vals) = self.ephemeral_priority.get(key) {
			return Some(vals);
		}
		self.main.get(key)
	}

	pub fn insert(&mut self, key: &str, val: Value) {
		_ = self.main.set(key, &[val]);
	}

	pub fn insert_multi<const N: usize>(&mut self, key: &str, vals: &[Value; N]) {
		_ = self.main.set(key, vals);
	}

	pub fn clear_ephemeral(&mut self) {
		self.ephemeral_new.clear();
		self.ephemeral_priority.clear();
		self.ephemeral_overlap.clear();
	}

	pub fn insert_ephemeral(&mut self, key: &str, val: Value) {
		_ = self.insert_ephemeral_multi(key, &[val]);
	}

	pub fn insert_ephemeral_multi<const N: usize>(&mut self, key: &str, vals: &[Value; N]) {
		match self.main.has(key) {
			false => match is_key_priority(key) {
				true => self.ephemeral_priority.set(key, vals),
				false => self.ephemeral_new.set(key, vals),
			},
			true => self.ephemeral_overlap.set(key, vals),
		}
	}
}

/// A key iterator for [`Map`]
pub struct MapKeyIter<'s> {
	map: &'s Map,
	main_idx: usize,
	ephemeral_new_idx: usize,
	ephemeral_priority_idx: usize,
}

impl<'i> MapKeyIter<'i> {
	pub fn new(map: &'i Map) -> Self {
		Self {
			map: map,
			main_idx: 0,
			ephemeral_new_idx: 0,
			ephemeral_priority_idx: 0,
		}
	}
}

impl<'i> Iterator for MapKeyIter<'i> {
	type Item = &'i str;

	fn next(&mut self) -> Option<Self::Item> {
		// iterate over priority ephemeral keys
		match self.map.ephemeral_priority.key_by_idx(self.ephemeral_priority_idx) {
			None => (),
			Some(key) => {
				self.ephemeral_priority_idx += 1;
				return Some(key);
			}
		}

		// iterate over main and ephemeral key overlaps
		match self.map.main.key_by_idx(self.main_idx) {
			None => (),
			Some(key) => {
				self.main_idx += 1;
				return Some(key);
			}
		}

		// iterate over the rest of ephemeral keys
		match self.map.ephemeral_new.key_by_idx(self.ephemeral_new_idx) {
			None => None,
			Some(key) => {
				self.ephemeral_new_idx += 1;
				Some(key)
			}
		}
	}
}

/// A key:value iterator for [`Map`].
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
	type Item = (&'i str, &'i [Value]);

	fn next(&mut self) -> Option<Self::Item> {
		// iterate over priority ephemeral KVs
		match self.map.ephemeral_priority.key_by_idx(self.ephemeral_priority_idx) {
			None => (),
			Some(key) => {
				let vals = self.map.ephemeral_priority.values_by_idx(self.ephemeral_priority_idx);
				self.ephemeral_priority_idx += 1;
				return Some((key, vals));
			}
		}

		// iterate over main KV and ephemeral value overlaps
		match self.map.main.key_by_idx(self.main_idx) {
			None => (),
			Some(key) => {
				let vals = match self.map.ephemeral_overlap.get(key) {
					Some(vs) => vs,
					None => self.map.main.values_by_idx(self.main_idx),
				};

				self.main_idx += 1;
				return Some((key, vals));
			}
		}

		// iterate over the rest of ephemeral KVs
		match self.map.ephemeral_new.key_by_idx(self.ephemeral_new_idx) {
			None => None,
			Some(key) => {
				let vals = self.map.ephemeral_new.values_by_idx(self.ephemeral_new_idx);
				self.ephemeral_new_idx += 1;
				Some((key, vals))
			}
		}
	}
}

impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut first: bool = true;
		for (key, vals) in self.iter() {
			write!(f, "{spacer}{key}={list_open}", spacer = if first { "" } else { " " }, list_open = if vals.len() > 1 { "[" } else { "" })?;
			for i in 0..vals.len() {
				let sep = if i != 0 { ", " } else { "" };
				write!(f, "{sep}{val}", val = vals[i])?;
			}
			if vals.len() > 1 {
				write!(f, "]")?;
			}
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

		kv.set("key_a", &[123.to_value()]);
		kv.set("key_b", &[456.to_value()]);
		kv.set("key_c", &[789.to_value(), "abc".to_value()]);
		kv.set("key_b", &["overwrites should not change key order".to_value()]);
		kv.set("error", &["priority keys should go first".to_value()]);

		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 5);
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
		assert_eq!(kv.store_size(), 0);

		kv.set("c", &[(-5678).to_value()]);
		kv.set("d", &[(9012.3456).to_value()]);
		kv.set("b", &[(1234).to_value()]);
		assert_eq!(kv.len(), 3);
		assert_eq!(kv.store_size(), 3);

		// overwrite existing key
		kv.set("d", &[(7890.1234).to_value()]);
		kv.set("error", &["first!".to_value()]);
		kv.set("e", &[Value::Size(7788), Value::Size(9900)]);
		kv.set("a", &["lalala".to_value()]);
		assert_eq!(kv.len(), 6);
		assert_eq!(kv.store_size(), 7);
	}

	#[test]
	fn key_overwrite() {
		let mut kv = KvStore::new();

		kv.set("a", &[1234.to_value(), (-5678).to_value()]);
		kv.set("b", &[("lalala").to_value()]);
		kv.set("c", &[true.to_value(), false.to_value(), true.to_value()]);
		kv.set("d", &[false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		// same size overwrite
		kv.set("b", &[(123.456).to_value()]);
		assert_eq!(kv.get("a").unwrap(), &[(1234).to_value(), (-5678).to_value()]);
		assert_eq!(kv.get("b").unwrap(), &[(123.456).to_value()]);
		assert_eq!(kv.get("c").unwrap(), &[true.to_value(), false.to_value(), true.to_value()]);
		assert_eq!(kv.get("d").unwrap(), &[false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		// overwrite with size increasee
		kv.set("b", &[1.to_value(), 2.to_value(), 3.to_value(), 4.to_value()]);
		assert_eq!(kv.get("a").unwrap(), &[(1234).to_value(), (-5678).to_value()]);
		assert_eq!(kv.get("b").unwrap(), &[1.to_value(), 2.to_value(), 3.to_value(), 4.to_value()]);
		assert_eq!(kv.get("c").unwrap(), &[true.to_value(), false.to_value(), true.to_value()]);
		assert_eq!(kv.get("d").unwrap(), &[false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 10);

		// overwrite with size decrease
		kv.set("c", &["lololo".to_value()]);
		assert_eq!(kv.get("a").unwrap(), &[(1234).to_value(), (-5678).to_value()]);
		assert_eq!(kv.get("b").unwrap(), &[1.to_value(), 2.to_value(), 3.to_value(), 4.to_value()]);
		assert_eq!(kv.get("c").unwrap(), &["lololo".to_value()]);
		assert_eq!(kv.get("d").unwrap(), &[false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 8);

		// modify the first and last attribute sizes to check edge handling
		kv.set("a", &[(1234).to_value()]);
		assert_eq!(kv.get("a").unwrap(), &[(1234).to_value()]);
		assert_eq!(kv.get("b").unwrap(), &[1.to_value(), 2.to_value(), 3.to_value(), 4.to_value()]);
		assert_eq!(kv.get("c").unwrap(), &["lololo".to_value()]);
		assert_eq!(kv.get("d").unwrap(), &[false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		kv.set("d", &[true.to_value(), false.to_value()]);
		assert_eq!(kv.get("a").unwrap(), &[(1234).to_value()]);
		assert_eq!(kv.get("b").unwrap(), &[1.to_value(), 2.to_value(), 3.to_value(), 4.to_value()]);
		assert_eq!(kv.get("c").unwrap(), &["lololo".to_value()]);
		assert_eq!(kv.get("d").unwrap(), &[true.to_value(), false.to_value()]);
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 8);
	}

	#[test]
	#[should_panic]
	fn insert_empty_key() {
		KvStore::new().set("", &["oh no".to_value()]);
	}

	#[test]
	#[should_panic]
	fn insert_empty_values() {
		KvStore::new().set("a_key", &[]);
	}

	#[test]
	#[should_panic]
	fn insert_invalid_key() {
		KvStore::new().set("no\twhitespace\tin\tkeys", &["please!".to_value()]);
	}

	#[test]
	#[should_panic]
	fn insert_restricted_key() {
		KvStore::new().set("level", &[55555i32.to_value()]);
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
		attr.insert_multi("key_c", &[789.to_value(), true.to_value()]);
		attr.insert_multi("key_b", &["overwrites should not change key order".to_value()]);
		attr.insert_multi("error", &["priority keys should go first".to_value()]);

		assert_eq!(attr.len(), 4);
		assert_eq!(attr.main.store_size(), 5);
		assert_eq!(
			attr.to_string(),
			"error=\"priority keys should go first\" key_a=123 key_b=\"overwrites should not change key order\" key_c=[789, true]"
		);
	}

	#[test]
	fn ephemeral_attributes() {
		let mut attr = Map::new();

		attr.insert_multi("key_a", &[123.to_value(), "lalala".to_value()]);
		attr.insert_multi("key_b", &[456.to_value(), "lololo".to_value()]);
		attr.insert("key_c", 789.to_value());
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrites should not change key order".to_value());
		attr.insert_ephemeral("key_d", "new key".to_value());
		attr.insert_ephemeral_multi("error", &["new".to_value(), "error".to_value()]);

		assert_eq!(attr.len(), 5);
		assert_eq!(attr.main.len(), 4);
		assert_eq!(attr.main.store_size(), 6);
		assert_eq!(attr.ephemeral_new.len(), 1);
		assert_eq!(attr.ephemeral_new.store_size(), 1);
		assert_eq!(attr.ephemeral_priority.len(), 0);
		assert_eq!(attr.ephemeral_priority.store_size(), 0);
		assert_eq!(attr.ephemeral_overlap.len(), 2);
		assert_eq!(attr.ephemeral_overlap.store_size(), 3);
		assert_eq!(
			attr.to_string(),
			"error=[\"new\", \"error\"] key_a=[123, \"lalala\"] key_b=\"overwrites should not change key order\" key_c=789 key_d=\"new key\"",
		);

		attr.clear_ephemeral();

		assert_eq!(attr.len(), 4);
		assert_eq!(attr.main.len(), 4);
		assert_eq!(attr.main.store_size(), 6);
		assert_eq!(attr.ephemeral_new.len(), 0);
		assert_eq!(attr.ephemeral_new.store_size(), 0);
		assert_eq!(attr.ephemeral_priority.len(), 0);
		assert_eq!(attr.ephemeral_priority.store_size(), 0);
		assert_eq!(attr.ephemeral_overlap.len(), 0);
		assert_eq!(attr.ephemeral_overlap.store_size(), 0);
		assert_eq!(attr.to_string(), "error=\"first error\" key_a=[123, \"lalala\"] key_b=[456, \"lololo\"] key_c=789",);
	}

	#[test]
	fn ephemeral_new_priority_keys() {
		let mut attr = Map::new();

		attr.insert_multi("key_a", &[123.to_value()]);
		attr.insert_multi("key_b", &[456.to_value(), true.to_value()]);
		attr.insert_multi("key_c", &[789.to_value()]);

		attr.insert_ephemeral_multi("error", &["oh".to_value(), "no!".to_value()]);
		attr.insert_ephemeral("key_d", "new key".to_value());

		assert_eq!(attr.len(), 5);
		assert_eq!(attr.main.len(), 3);
		assert_eq!(attr.main.store_size(), 4);
		assert_eq!(attr.ephemeral_new.len(), 1);
		assert_eq!(attr.ephemeral_new.store_size(), 1);
		assert_eq!(attr.ephemeral_priority.len(), 1);
		assert_eq!(attr.ephemeral_priority.store_size(), 2);
		assert_eq!(attr.ephemeral_overlap.len(), 0);
		assert_eq!(attr.ephemeral_overlap.store_size(), 0);
		assert_eq!(attr.to_string(), "error=[\"oh\", \"no!\"] key_a=123 key_b=[456, true] key_c=789 key_d=\"new key\"",);

		attr.clear_ephemeral();

		assert_eq!(attr.len(), 3);
		assert_eq!(attr.main.len(), 3);
		assert_eq!(attr.main.store_size(), 4);
		assert_eq!(attr.ephemeral_new.len(), 0);
		assert_eq!(attr.ephemeral_new.store_size(), 0);
		assert_eq!(attr.ephemeral_priority.len(), 0);
		assert_eq!(attr.ephemeral_priority.store_size(), 0);
		assert_eq!(attr.ephemeral_overlap.len(), 0);
		assert_eq!(attr.ephemeral_overlap.store_size(), 0);
		assert_eq!(attr.to_string(), "key_a=123 key_b=[456, true] key_c=789",);
	}

	#[test]
	fn iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert_multi("key_b", &[456.to_value(), true.to_value()]);
		attr.insert_multi("key_c", &[789.to_value(), false.to_value()]);
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrite!".to_value());
		attr.insert_ephemeral_multi("key_d", &["new".to_value(), " key".to_value()]);
		attr.insert_ephemeral("error", "new error".to_value());

		let mut got: Vec<(&str, Vec<Value>)> = Vec::new();
		for kvs in attr.iter() {
			got.push((kvs.0, kvs.1.to_vec()));
		}

		assert_eq!(
			got.as_array::<5>().expect("invalid number of keys"),
			&[
				("error", vec!["new error".to_value()]),
				("key_a", vec![123.to_value()]),
				("key_b", vec!["overwrite!".to_value()]),
				("key_c", vec![789.to_value(), false.to_value()]),
				("key_d", vec!["new".to_value(), " key".to_value()]),
			]
		);
	}

	#[test]
	fn key_iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert_multi("key_b", &[456.to_value(), true.to_value()]);
		attr.insert_multi("key_c", &[789.to_value(), false.to_value()]);
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrite!".to_value());
		attr.insert_ephemeral_multi("key_d", &["new".to_value(), " key".to_value()]);
		attr.insert_ephemeral("error", "new error".to_value());

		let mut got_keys: Vec<&str> = Vec::new();
		for key in attr.key_iter() {
			got_keys.push(key);
		}

		assert_eq!(got_keys.as_array::<5>().expect("invalid number of keys"), &["error", "key_a", "key_b", "key_c", "key_d"]);
	}
}
