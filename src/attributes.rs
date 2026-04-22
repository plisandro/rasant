pub mod scalar;
pub mod value;

use std::fmt;

use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP};

pub use scalar::{Scalar, ToScalar};
pub use value::{ToValue, Value};

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
struct ScalarKvStore {
	keys: String,
	scalars: Vec<Scalar>,
	key_idxs: Vec<(usize, usize)>,
	scalar_idxs: Vec<(usize, usize)>,
}

impl ScalarKvStore {
	fn new() -> Self {
		Self {
			keys: String::new(),
			scalars: Vec::new(),
			key_idxs: Vec::new(),
			scalar_idxs: Vec::new(),
		}
	}

	fn clear(&mut self) {
		self.keys.clear();
		self.scalars.clear();
		self.key_idxs.clear();
		self.scalar_idxs.clear();
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

	fn scalars_by_idx(&self, i: usize) -> &[Scalar] {
		let (start, end) = self.scalar_idxs[i];
		&self.scalars[start..end + 1]
	}

	fn value_by_idx(&self, i: usize) -> Value<'_> {
		let (start, end) = self.scalar_idxs[i];

		// TODO: fix me
		if start == end {
			Value::Scalar(self.scalars[start].clone())
		} else {
			Value::Set(&self.scalars[start..end + 1])
		}
	}

	fn len(&self) -> usize {
		self.key_idxs.len()
	}

	fn store_size(&self) -> usize {
		self.scalars.len()
	}

	fn has(&self, key: &str) -> bool {
		self.key_to_idx(key).is_some()
	}

	fn get(&self, key: &str) -> Option<&[Scalar]> {
		match self.key_to_idx(key) {
			Some(i) => Some(self.scalars_by_idx(i)),
			None => None,
		}
	}

	fn set(&mut self, key: &str, ss: &[Scalar]) {
		if key.len() == 0 {
			panic!("empty log attribute key {{\"\" -> {ss:?}}}");
		}
		if ss.len() == 0 {
			panic!("empty log attribute scalar value {{\"{key}\" -> {ss:?}}}");
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {ss:?}}}");
		}
		if is_key_restricted(key) {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {ss:?}}}");
		}

		match self.key_to_idx(key) {
			Some(i) => {
				// overwrite existing key
				let (pre_start, pre_end) = self.scalar_idxs[i];
				let pre_size = pre_end - pre_start + 1;
				match ss.len() == pre_size {
					true => {
						// yay, new values fit in the existing slot
						// TODO: is there any way to copy instead?
						self.scalars[pre_start..pre_end + 1].clone_from_slice(ss);
					}
					false => {
						// we need to resize :'(
						for (start, end) in &mut self.scalar_idxs {
							if *start >= pre_size && *start > pre_start {
								*start -= pre_size;
								*end -= pre_size;
							}
						}

						self.scalars.drain(pre_start..pre_end + 1);
						let start_idx = self.scalars.len();
						let end_idx = start_idx + ss.len() - 1;
						self.scalars.extend_from_slice(ss);
						self.scalar_idxs[i] = (start_idx, end_idx);
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

					let start_idx = self.scalars.len();
					let end_idx = start_idx + ss.len() - 1;
					self.scalars.extend_from_slice(ss);
					self.scalar_idxs.insert(0, (start_idx, end_idx));
				} else {
					// insert new key last
					let key_start = self.keys.len();
					let key_end = key_start + key_len - 1;
					self.key_idxs.push((key_start, key_end));
					self.keys.push_str(key);

					let start_idx = self.scalars.len();
					let end_idx = start_idx + ss.len() - 1;
					self.scalars.extend_from_slice(ss);
					self.scalar_idxs.push((start_idx, end_idx));
				}
			}
		}
	}
}

#[derive(Clone, Debug)]
pub struct Map {
	main: ScalarKvStore,
	ephemeral_new: ScalarKvStore,
	ephemeral_priority: ScalarKvStore,
	ephemeral_overlap: ScalarKvStore,
}

impl Map {
	pub fn new() -> Self {
		Self {
			main: ScalarKvStore::new(),
			ephemeral_new: ScalarKvStore::new(),
			ephemeral_priority: ScalarKvStore::new(),
			ephemeral_overlap: ScalarKvStore::new(),
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

	pub fn get(&self, key: &str) -> Option<Value<'_>> {
		let scalars: Option<&[Scalar]>;
		if let Some(ss) = self.ephemeral_new.get(key) {
			scalars = Some(ss);
		} else if let Some(ss) = self.ephemeral_priority.get(key) {
			scalars = Some(ss);
		} else {
			scalars = self.main.get(key);
		}

		match scalars {
			None => None,
			// TODO: fix me
			Some(ss) => Some(if ss.len() == 1 { Value::Scalar(ss[0].clone()) } else { Value::Set(ss) }),
		}
	}

	pub fn insert(&mut self, key: &str, val: Value) {
		let ss = match val {
			Value::Scalar(s) => &[s][..],
			Value::Set(ss) => ss,
		};
		_ = self.main.set(key, ss);
	}

	pub fn clear_ephemeral(&mut self) {
		self.ephemeral_new.clear();
		self.ephemeral_priority.clear();
		self.ephemeral_overlap.clear();
	}

	pub fn insert_ephemeral(&mut self, key: &str, val: Value) {
		let ss = match val {
			Value::Scalar(s) => &[s][..],
			Value::Set(ss) => ss,
		};

		match self.main.has(key) {
			false => match is_key_priority(key) {
				true => self.ephemeral_priority.set(key, ss),
				false => self.ephemeral_new.set(key, ss),
			},
			true => self.ephemeral_overlap.set(key, ss),
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
	type Item = (&'i str, Value<'i>);

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
				let val: Value;
				if let Some(s) = self.map.ephemeral_overlap.get(key) {
					// TODO: fix me
					val = if s.len() == 1 { Value::Scalar(s[0].clone()) } else { Value::Set(s) };
					//val = s.to_value();
				} else {
					val = self.map.main.value_by_idx(self.main_idx);
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
		for (key, val) in self.iter() {
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

	#[test]
	fn indexed_keys_order() {
		let mut kv = ScalarKvStore::new();

		kv.set("key_a", [Scalar::Int(123)].as_slice());
		kv.set("key_b", [Scalar::Int(456)].as_slice());
		kv.set("key_c", [Scalar::Int(789), Scalar::String("abc".into())].as_slice());
		kv.set("key_b", [Scalar::String("overwrites should not change key order".into())].as_slice());
		kv.set("error", [Scalar::String("priority keys should go first".into())].as_slice());

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
		let mut kv = ScalarKvStore::new();

		assert_eq!(kv.len(), 0);
		assert_eq!(kv.store_size(), 0);

		kv.set("c", [Scalar::Int(-5678)].as_slice());
		kv.set("d", [Scalar::Float(9012.3456)].as_slice());
		kv.set("b", [Scalar::Int(1234)].as_slice());
		assert_eq!(kv.len(), 3);
		assert_eq!(kv.store_size(), 3);

		// overwrite existing key
		kv.set("d", [Scalar::Float(7890.1234)].as_slice());
		kv.set("error", [Scalar::String("first!".into())].as_slice());
		kv.set("e", [Scalar::Size(7788), Scalar::Size(9900)].as_slice());
		kv.set("a", [Scalar::String("lalala".into())].as_slice());
		assert_eq!(kv.len(), 6);
		assert_eq!(kv.store_size(), 7);
	}

	#[test]
	fn key_overwrite() {
		let mut kv = ScalarKvStore::new();

		kv.set("a", [Scalar::Int(1234), Scalar::Int(-5678)].as_slice());
		kv.set("b", [Scalar::String("lalala".into())].as_slice());
		kv.set("c", [Scalar::Bool(true), Scalar::Bool(false), Scalar::Bool(true)].as_slice());
		kv.set("d", [Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		// same size overwrite
		kv.set("b", &[Scalar::Float(123.456)].as_slice());
		assert_eq!(kv.get("a").unwrap(), [Scalar::Int(1234), Scalar::Int(-5678)].as_slice());
		assert_eq!(kv.get("b").unwrap(), [Scalar::Float(123.456)].as_slice());
		assert_eq!(kv.get("c").unwrap(), [Scalar::Bool(true), Scalar::Bool(false), Scalar::Bool(true)].as_slice());
		assert_eq!(kv.get("d").unwrap(), [Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		// overwrite with size increasee
		kv.set("b", &[Scalar::Int(1), Scalar::Int(2), Scalar::Int(3), Scalar::Int(4)].as_slice());
		assert_eq!(kv.get("a").unwrap(), [Scalar::Int(1234), Scalar::Int(-5678)].as_slice());
		assert_eq!(kv.get("b").unwrap(), [Scalar::Int(1), Scalar::Int(2), Scalar::Int(3), Scalar::Int(4)].as_slice());
		assert_eq!(kv.get("c").unwrap(), [Scalar::Bool(true), Scalar::Bool(false), Scalar::Bool(true)].as_slice());
		assert_eq!(kv.get("d").unwrap(), [Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 10);

		// overwrite with size decrease
		kv.set("c", &[Scalar::String("lololo".into())].as_slice());
		assert_eq!(kv.get("a").unwrap(), [Scalar::Int(1234), Scalar::Int(-5678)].as_slice());
		assert_eq!(kv.get("b").unwrap(), [Scalar::Int(1), Scalar::Int(2), Scalar::Int(3), Scalar::Int(4)].as_slice());
		assert_eq!(kv.get("c").unwrap(), [Scalar::String("lololo".into())].as_slice());
		assert_eq!(kv.get("d").unwrap(), [Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 8);

		// modify the first and last attribute sizes to check edge handling
		kv.set("a", &[Scalar::Int(1234)].as_slice());
		assert_eq!(kv.get("a").unwrap(), [Scalar::Int(1234)].as_slice());
		assert_eq!(kv.get("b").unwrap(), [Scalar::Int(1), Scalar::Int(2), Scalar::Int(3), Scalar::Int(4)].as_slice());
		assert_eq!(kv.get("c").unwrap(), [Scalar::String("lololo".into())].as_slice());
		assert_eq!(kv.get("d").unwrap(), [Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 7);

		kv.set("d", &[Scalar::Bool(true), Scalar::Bool(false)].as_slice());
		assert_eq!(kv.get("a").unwrap(), [Scalar::Int(1234)].as_slice());
		assert_eq!(kv.get("b").unwrap(), [Scalar::Int(1), Scalar::Int(2), Scalar::Int(3), Scalar::Int(4)].as_slice());
		assert_eq!(kv.get("c").unwrap(), [Scalar::String("lololo".into())].as_slice());
		assert_eq!(kv.get("d").unwrap(), [Scalar::Bool(true), Scalar::Bool(false)].as_slice());
		assert_eq!(kv.len(), 4);
		assert_eq!(kv.store_size(), 8);
	}

	#[test]
	#[should_panic]
	fn insert_empty_key() {
		ScalarKvStore::new().set("", [Scalar::String("oh no".into())].as_slice());
	}

	#[test]
	#[should_panic]
	fn insert_empty_values() {
		ScalarKvStore::new().set("a_key", [].as_slice());
	}

	#[test]
	#[should_panic]
	fn insert_invalid_key() {
		ScalarKvStore::new().set("no\twhitespace\tin\tkeys", [Scalar::String("please!".into())].as_slice());
	}

	#[test]
	#[should_panic]
	fn insert_restricted_key() {
		ScalarKvStore::new().set("level", [Scalar::Int(55555)].as_slice());
	}
}

#[cfg(test)]
mod map_tests {
	use super::*;
	use crate::attributes::{ToScalar, ToValue};

	#[test]
	fn indexed_keys_order() {
		let mut attr = Map::new();

		attr.insert("key_a", (123 as i32).to_value());
		attr.insert("key_b", (456 as i32).to_value());
		attr.insert("key_c", [Scalar::Int(789), Scalar::Bool(true)].to_value());
		attr.insert("key_b", "overwrites should not change key order".to_value());
		attr.insert("error", "priority keys should go first".to_value());

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

		attr.insert("key_a", [123.to_scalar(), "lalala".to_scalar()].to_value());
		attr.insert("key_b", [456.to_scalar(), "lololo".to_scalar()].to_value());
		attr.insert("key_c", 789.to_value());
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrites should not change key order".to_value());
		attr.insert_ephemeral("key_d", "new key".to_value());
		attr.insert_ephemeral("error", ["new".to_scalar(), "error".to_scalar()].to_value());

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

		attr.insert("key_a", 123.to_value());
		attr.insert("key_b", [456.to_scalar(), true.to_scalar()].to_value());
		attr.insert("key_c", 789.to_value());

		attr.insert_ephemeral("error", ["oh".to_scalar(), "no!".to_scalar()].to_value());
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
		attr.insert("key_b", [456.to_scalar(), true.to_scalar()].to_value());
		attr.insert("key_c", [789.to_scalar(), false.to_scalar()].to_value());
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrite!".to_value());
		attr.insert_ephemeral("key_d", ["new".to_scalar(), " key".to_scalar()].to_value());
		attr.insert_ephemeral("error", "new error".to_value());

		let mut got: Vec<(&str, Value)> = Vec::new();
		for kvs in attr.iter() {
			got.push((kvs.0, kvs.1));
		}

		assert_eq!(
			got.as_array::<5>().expect("invalid number of keys"),
			&[
				("error", "new error".to_value()),
				("key_a", 123.to_value()),
				("key_b", "overwrite!".to_value()),
				("key_c", [789.to_scalar(), false.to_scalar()].to_value()),
				("key_d", ["new".to_scalar(), " key".to_scalar()].to_value()),
			]
		);
	}

	#[test]
	fn key_iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", 123.to_value());
		attr.insert("key_b", [456.to_scalar(), true.to_scalar()].to_value());
		attr.insert("key_c", [789.to_scalar(), false.to_scalar()].to_value());
		attr.insert("error", "first error".to_value());

		attr.insert_ephemeral("key_b", "overwrite!".to_value());
		attr.insert_ephemeral("key_d", ["new".to_scalar(), " key".to_scalar()].to_value());
		attr.insert_ephemeral("error", "new error".to_value());

		let mut got_keys: Vec<&str> = Vec::new();
		for key in attr.key_iter() {
			got_keys.push(key);
		}

		assert_eq!(got_keys.as_array::<5>().expect("invalid number of keys"), &["error", "key_a", "key_b", "key_c", "key_d"]);
	}
}
