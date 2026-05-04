pub mod scalar;
pub mod value;

use std::fmt;
use std::slice;

use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP};

// TODO: fix imports;
#[allow(unused_imports)]
pub use scalar::Scalar;
pub use value::Value;

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

/// A store for a ordered map of (key -> [`Value`])
#[derive(Clone, Debug)]
pub struct Map {
	/// A container for all keys in this store
	keys: String,
	/// String indeces for every key: [start, end)
	key_str_idxs: Vec<(usize, usize)>,
	/// A container for all Scalars used by Values in this store.
	scalar_pool: Vec<Scalar>,
	/// Indexes for 1st set of Scalar's associated with each key, as [start, end)
	scalar_1_idxs: Vec<(usize, usize)>,
	/// Indexes for optional 2st set of Scalar's associated with each key, as [start, end);
	/// (0.0) indicates no 2nd set present.
	scalar_2_idxs: Vec<(usize, usize)>,
}

impl Map {
	pub fn new() -> Self {
		Self {
			keys: String::new(),
			key_str_idxs: Vec::new(),
			scalar_pool: Vec::new(),
			scalar_1_idxs: Vec::new(),
			scalar_2_idxs: Vec::new(),
		}
	}

	pub fn clear(&mut self) {
		self.keys.clear();
		self.key_str_idxs.clear();
		self.scalar_pool.clear();
		self.scalar_1_idxs.clear();
		self.scalar_2_idxs.clear();
	}

	pub fn copy_from(&mut self, other: &Self) {
		self.keys.clear();
		self.key_str_idxs.clear();
		self.scalar_pool.clear();
		self.scalar_1_idxs.clear();
		self.scalar_2_idxs.clear();

		if !other.is_empty() {
			self.keys.push_str(&other.keys);
			self.key_str_idxs.extend(&other.key_str_idxs);
			// iterating over scalar_pool yields &Clone instead of Clone >:(
			self.scalar_pool.extend_from_slice(&other.scalar_pool);
			self.scalar_1_idxs.extend(&other.scalar_1_idxs);
			self.scalar_2_idxs.extend(&other.scalar_2_idxs);
		}
	}

	fn len(&self) -> usize {
		self.key_str_idxs.len()
	}

	fn is_empty(&self) -> bool {
		self.key_str_idxs.is_empty()
	}

	fn store_size(&self) -> usize {
		self.scalar_pool.len()
	}

	pub fn key_iter(&self) -> MapKeyIter<'_> {
		MapKeyIter::new(self)
	}

	pub fn iter(&self) -> MapIter<'_> {
		MapIter::new(self)
	}

	// for key strings (relatively short, and small in number), a linear
	// search turns out to be the most efficient way to seek.
	fn idx_by_key(&self, key: &str) -> Option<usize> {
		let key_size = key.len();
		for i in 0..self.key_str_idxs.len() {
			let (key_start, key_end) = self.key_str_idxs[i];
			if (key_end - key_start) != key_size {
				continue;
			}
			if key == &self.keys[key_start..key_end] {
				return Some(i);
			}
		}

		None
	}

	fn key_by_idx(&self, idx: usize) -> Option<&str> {
		match idx < self.key_str_idxs.len() {
			false => None,
			true => {
				let (start, end) = self.key_str_idxs[idx];
				Some(&self.keys[start..end])
			}
		}
	}

	fn value_by_idx(&self, idx: usize) -> Value<'_> {
		match self.scalar_2_idxs[idx] {
			(0, 0) => {
				// a single Scalar or Set
				let (start, end) = self.scalar_1_idxs[idx];

				if start == end - 1 {
					// TODO: fix me
					Value::Scalar(self.scalar_pool[start].clone())
				} else {
					Value::List(&self.scalar_pool[start..end])
				}
			}
			(start_2, end_2) => {
				// a Map
				let (start_1, end_1) = self.scalar_1_idxs[idx];

				Value::Map(&self.scalar_pool[start_1..end_1], &self.scalar_pool[start_2..end_2])
			}
		}
	}

	pub fn has(&self, key: &str) -> bool {
		self.idx_by_key(key).is_some()
	}

	fn has_idx(&self, idx: usize) -> bool {
		idx < self.key_str_idxs.len()
	}

	fn scalar_pool_add(&mut self, insert_first: bool, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let start_1 = self.scalar_pool.len();
		let end_1 = start_1 + ss_1.len();
		let (start_2, end_2) = match ss_2.is_empty() {
			true => (0, 0),
			false => (end_1, end_1 + ss_2.len()),
		};

		self.scalar_pool.extend_from_slice(ss_1);
		self.scalar_pool.extend_from_slice(ss_2);

		match insert_first {
			true => {
				self.scalar_1_idxs.insert(0, (start_1, end_1));
				self.scalar_2_idxs.insert(0, (start_2, end_2));
			}
			false => {
				self.scalar_1_idxs.push((start_1, end_1));
				self.scalar_2_idxs.push((start_2, end_2));
			}
		}
	}

	// TODO: this function is too verbose, rewrite.
	fn scalar_pool_replace(&mut self, idx: usize, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let (pre_start_1, pre_end_1) = self.scalar_1_idxs[idx];
		let pre_size_1 = pre_end_1 - pre_start_1;

		// delete slot for first slice
		if ss_1.len() == pre_size_1 {
			// yay, new scalars fit in the existing slot
			// TODO: is there any way to copy instead?
			self.scalar_pool[pre_start_1..pre_end_1].clone_from_slice(ss_1);
		} else {
			// we'll have to resize and extend :'(
			self.scalar_pool.drain(pre_start_1..pre_end_1);

			self.scalar_1_idxs.iter_mut().for_each(|(start, end)| {
				if *start >= pre_size_1 && *start > pre_start_1 {
					*start -= pre_size_1;
					*end -= pre_size_1;
				}
			});
			self.scalar_2_idxs.iter_mut().for_each(|(start, end)| {
				if *start >= pre_size_1 && *start > pre_start_1 {
					*start -= pre_size_1;
					*end -= pre_size_1;
				}
			});

			let start_1 = self.scalar_pool.len();
			let end_1 = start_1 + ss_1.len();

			self.scalar_pool.extend_from_slice(ss_1);
			self.scalar_1_idxs[idx] = (start_1, end_1);
		}

		// delete slot for second slice, if present
		let (pre_start_2, pre_end_2) = self.scalar_2_idxs[idx];
		let pre_size_2 = pre_end_2 - pre_start_2;

		if ss_2.len() == pre_size_2 {
			self.scalar_pool[pre_start_2..pre_end_2].clone_from_slice(ss_2);
		} else {
			// we'll have to resize and extend :'(
			self.scalar_pool.drain(pre_start_2..pre_end_2);

			self.scalar_1_idxs.iter_mut().for_each(|(start, end)| {
				if *start >= pre_size_2 && *start > pre_start_2 {
					*start -= pre_size_2;
					*end -= pre_size_2;
				}
			});
			self.scalar_2_idxs.iter_mut().for_each(|(start, end)| {
				if *start >= pre_size_2 && *start > pre_start_2 {
					*start -= pre_size_2;
					*end -= pre_size_2;
				}
			});

			let start_2 = self.scalar_pool.len();
			let end_2 = start_2 + ss_2.len();

			self.scalar_pool.extend_from_slice(ss_2);
			self.scalar_2_idxs[idx] = (start_2, end_2);
		}
	}

	pub fn get(&self, key: &str) -> Option<Value<'_>> {
		match self.idx_by_key(key) {
			Some(i) => Some(self.value_by_idx(i)),
			None => None,
		}
	}

	fn set(&mut self, key: &str, val: &Value) {
		if key.is_empty() {
			panic!("empty log attribute key {{\"\" -> {val:?}}}");
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {val:?}}}");
		}
		if is_key_restricted(key) {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {val:?}}}");
		}

		let (ss_1, ss_2): (&[Scalar], &[Scalar]) = match val {
			Value::Scalar(s) => (slice::from_ref(s), &[]),
			Value::List(ss) => (*ss, &[]),
			Value::Map(keys, vals) => {
				if keys.len() != vals.len() {
					panic!("Map scalars mismatch for attribute key {{\"{key}\" -> {val:?}}}");
				}
				(*keys, *vals)
			}
		};
		if ss_1.is_empty() {
			panic!("no scalars for attribute key {{\"{key}\" -> {val:?}}}");
		}

		if let Some(i) = self.idx_by_key(key) {
			// overwrite existing key
			self.scalar_pool_replace(i, ss_1, ss_2);
			return;
		}

		// insert new key
		match is_key_priority(key) {
			true => {
				// priority keys are inserted first...
				self.scalar_pool_add(true, ss_1, ss_2);

				let key_len = key.len();

				// ...which means shifting all existing key idxs references :'(
				self.key_str_idxs.iter_mut().for_each(|(start, end)| {
					*start += key_len;
					*end += key_len;
				});

				self.keys.insert_str(0, key);
				self.key_str_idxs.insert(0, (0, key_len));
			}
			false => {
				// insert new key last
				self.scalar_pool_add(false, ss_1, ss_2);

				let key_start = self.keys.len();
				let key_end = key_start + key.len();

				self.keys.push_str(key);
				self.key_str_idxs.push((key_start, key_end));
			}
		}
	}

	pub fn insert_ref(&mut self, key: &str, val: &Value) {
		self.set(key, val);
	}

	pub fn insert(&mut self, key: &str, val: Value) {
		self.set(key, &val);
	}
}

/// A key iterator for [`Map`]
pub struct MapKeyIter<'s> {
	map: &'s Map,
	idx: usize,
}

impl<'i> MapKeyIter<'i> {
	pub fn new(map: &'i Map) -> Self {
		Self { map: map, idx: 0 }
	}
}

impl<'i> Iterator for MapKeyIter<'i> {
	type Item = &'i str;

	fn next(&mut self) -> Option<Self::Item> {
		let key = self.map.key_by_idx(self.idx);
		self.idx += 1;

		key
	}
}

/// A key:value iterator for [`Map`].
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
	type Item = (&'i str, Value<'i>);

	fn next(&mut self) -> Option<Self::Item> {
		match self.map.key_by_idx(self.idx) {
			None => None,
			Some(key) => {
				let val = self.map.value_by_idx(self.idx);
				self.idx += 1;

				Some((key, val))
			}
		}
	}
}

impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut first: bool = true;
		for (key, val) in self.iter() {
			write!(f, "{spacer}{key}={val}", spacer = if first { "" } else { " " })?;
			first = false;
		}

		Ok(())
	}
}

/* ----------------------- Tests ----------------------- */

// TODO: add tests for unordered kv stores
#[cfg(test)]
mod map {
	use super::*;

	#[test]
	fn indexed_keys_order() {
		let mut attr = Map::new();

		attr.set("key_a", &Value::from(123));
		attr.set("key_b", &Value::from(456));
		attr.set("key_c", &Value::from(&[Scalar::from(789), Scalar::from("abc")]));
		attr.set("key_b", &Value::from("overwrites should not change key order"));
		attr.set("error", &Value::from("priority keys should go first"));

		dbg!(&attr);
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 5);
		assert_eq!(attr.idx_by_key("error"), Some(0));
		assert_eq!(attr.idx_by_key("key_a"), Some(1));
		assert_eq!(attr.idx_by_key("key_b"), Some(2));
		assert_eq!(attr.idx_by_key("key_c"), Some(3));
		assert_eq!(attr.idx_by_key("bad_key"), None);
	}

	#[test]
	fn basic_operations() {
		let mut attr = Map::new();

		assert_eq!(attr.len(), 0);
		assert_eq!(attr.store_size(), 0);

		attr.set("c", &Value::from(-5678));
		attr.set("d", &Value::from(9012.3456));
		attr.set("b", &Value::from(1234));
		assert_eq!(attr.len(), 3);
		assert_eq!(attr.store_size(), 3);
		assert_eq!(attr.to_string(), "c=-5678 d=9012.3456 b=1234");

		// overwrite existing key
		attr.set("d", &Value::from(7890.1234));
		attr.set("error", &Value::from("first!"));
		attr.set("e", &Value::from(&[Scalar::from(7788 as usize), Scalar::from(9900)]));
		attr.set("a", &Value::Scalar(Scalar::String("lalala".into())));
		assert_eq!(attr.len(), 6);
		assert_eq!(attr.store_size(), 7);
		assert_eq!(attr.to_string(), "error=\"first!\" c=-5678 d=7890.1234 b=1234 e=[0x1e6c, 9900] a=\"lalala\"");
	}

	#[test]
	fn key_overwrite() {
		let mut attr = Map::new();

		attr.set("a", &Value::from(&[Scalar::from(1234), Scalar::from(-5678)]));
		attr.set("b", &Value::from("lalala"));
		attr.set("c", &Value::from(&[Scalar::from(true), Scalar::from(false), Scalar::from(true)]));
		attr.set("d", &Value::from(false));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 7);
		assert_eq!(attr.to_string(), "a=[1234, -5678] b=\"lalala\" c=[true, false, true] d=false");

		// same size overwrite
		attr.set("b", &Value::from(123.456));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 7);
		assert_eq!(attr.to_string(), "a=[1234, -5678] b=123.456 c=[true, false, true] d=false");

		// overwrite with size increasee
		attr.set("b", &Value::from(&[Scalar::from(1), Scalar::from(2), Scalar::from(3), Scalar::from(4)]));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 10);
		assert_eq!(attr.to_string(), "a=[1234, -5678] b=[1, 2, 3, 4] c=[true, false, true] d=false");

		// overwrite with size decrease
		attr.set("c", &Value::from("lololo"));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 8);
		assert_eq!(attr.to_string(), "a=[1234, -5678] b=[1, 2, 3, 4] c=\"lololo\" d=false");

		// modify the first and last attribute sizes to check edge handling
		attr.set("a", &Value::from(1234));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 7);
		assert_eq!(attr.to_string(), "a=1234 b=[1, 2, 3, 4] c=\"lololo\" d=false");

		attr.set("d", &Value::from((&[Scalar::from("sub_a"), Scalar::from("sub_b")], &[Scalar::from(true), Scalar::from(false)])));
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.store_size(), 10);
		assert_eq!(attr.to_string(), "a=1234 b=[1, 2, 3, 4] c=\"lololo\" d={\"sub_a\": true, \"sub_b\": false}");
	}

	#[test]
	#[should_panic]
	fn insert_empty_key() {
		Map::new().set("", &Value::from("oh no"));
	}

	#[test]
	#[should_panic]
	fn insert_invalid_key() {
		Map::new().set("no whitespace\tin\tkeys", &Value::from("please!"));
	}

	#[test]
	#[should_panic]
	fn insert_restricted_key() {
		Map::new().set("level", &Value::from(55555));
	}

	#[test]
	#[should_panic]
	fn insert_empty_list() {
		Map::new().set("a_key", &Value::from(&[]));
	}

	#[test]
	#[should_panic]
	fn invalid_empty_map() {
		Map::new().set("wrong_map", &Value::from((&[], &[])));
	}

	#[test]
	#[should_panic]
	fn insert_invalid_map() {
		Map::new().set(
			"wrong_map",
			&Value::Map(
				[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_c")].as_slice(),
				[Scalar::from(123), Scalar::from("oh no")].as_slice(),
			),
		);
	}

	#[test]
	fn iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", Value::from(123));
		attr.insert("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)]));
		attr.insert("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)]));
		attr.insert("error", Value::from("an error"));
		attr.insert("key_d", Value::from(&[Scalar::from("new"), Scalar::from("key")]));

		let mut got: Vec<(&str, Value)> = Vec::new();
		for kvs in attr.iter() {
			got.push((kvs.0, kvs.1));
		}

		assert_eq!(
			got.as_array::<5>().expect("invalid number of results"),
			&[
				("error", Value::from("an error")),
				("key_a", Value::from(123)),
				("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)])),
				("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)])),
				("key_d", Value::from(&[Scalar::from("new"), Scalar::from("key")])),
			]
		);
	}

	#[test]
	fn key_iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", Value::from(123));
		attr.insert("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)]));
		attr.insert("error", Value::from("first error"));
		attr.insert("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)]));
		attr.insert("key_d", Value::from(3.14159));

		let mut got_keys: Vec<&str> = Vec::new();
		for key in attr.key_iter() {
			got_keys.push(key);
		}

		assert_eq!(got_keys.as_array::<5>().expect("invalid number of keys"), &["error", "key_a", "key_b", "key_c", "key_d"]);
	}
}
