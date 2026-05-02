pub mod scalar;
pub mod value;

use std::fmt;
use std::slice;

use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP};
use crate::types::AttributeStringSeek;

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
#[derive(Debug, Clone)]
pub struct Map {
	/// A container for all strings in this map (keys and scalars)
	string_pool: String,
	/// Indeces for every string in the pool: [start, end)
	string_idxs: Vec<(usize, usize)>,
	/// String indeces for all keys.
	key_idxs: Vec<usize>,
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
			string_pool: String::new(),
			string_idxs: Vec::new(),
			key_idxs: Vec::new(),
			scalar_pool: Vec::new(),
			scalar_1_idxs: Vec::new(),
			scalar_2_idxs: Vec::new(),
		}
	}

	pub fn clear(&mut self) {
		self.string_pool.clear();
		self.string_idxs.clear();
		self.key_idxs.clear();
		self.scalar_pool.clear();
		self.scalar_1_idxs.clear();
		self.scalar_2_idxs.clear();
	}

	pub fn copy_from(&mut self, other: &Self) {
		self.string_pool.clear();
		self.string_idxs.clear();
		self.key_idxs.clear();
		self.scalar_pool.clear();
		self.scalar_1_idxs.clear();
		self.scalar_2_idxs.clear();

		if !other.is_empty() {
			self.string_pool.push_str(&other.string_pool);
			self.string_idxs.extend(&other.string_idxs);
			self.key_idxs.extend(&other.key_idxs);
			// iterating over scalar_pool yields &Clone instead of Clone >:(
			self.scalar_pool.extend_from_slice(&other.scalar_pool);
			self.scalar_1_idxs.extend(&other.scalar_1_idxs);
			self.scalar_2_idxs.extend(&other.scalar_2_idxs);
		}
	}

	fn len(&self) -> usize {
		self.key_idxs.len()
	}

	fn is_empty(&self) -> bool {
		self.key_idxs.is_empty()
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
		for i in 0..self.key_idxs.len() {
			let (key_start, key_end) = self.string_idxs[self.key_idxs[i]];
			if (key_end - key_start) != key_size {
				continue;
			}
			if key == &self.string_pool[key_start..key_end] {
				return Some(i);
			}
		}

		None
	}

	fn key_by_idx(&self, idx: usize) -> Option<&str> {
		match idx < self.key_idxs.len() {
			false => None,
			true => {
				let (start, end) = self.string_idxs[self.key_idxs[idx]];
				Some(&self.string_pool[start..end])
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
		idx < self.key_idxs.len()
	}

	fn string_pool_add(&mut self, s: &str) -> usize {
		let start = self.string_pool.len();
		let end = start + s.len();
		let idx = self.string_idxs.len();

		self.string_pool.push_str(s);
		self.string_idxs.push((start, end));

		idx
	}

	fn string_pool_remove(&mut self, del_idx: usize) {
		let (del_start, del_end) = self.string_idxs[del_idx];
		let del_size = del_end - del_start;
		self.string_pool.drain(del_start..del_end);
		self.string_idxs.remove(del_idx);

		// Re-align all indexed string entries...
		self.string_idxs.iter_mut().for_each(|(start, end)| {
			if *start >= del_size && *start >= del_start {
				*start -= del_size;
				*end -= del_size;
			}
		});

		// ...and all items refering to strings by idx.
		self.key_idxs.iter_mut().for_each(|idx| {
			if *idx > 0 && *idx >= del_idx {
				*idx -= 1;
			}
		});
		self.scalar_pool.iter_mut().for_each(|v| {
			if let Scalar::String(s) = v {
				s.realign_by_deleted_idx(del_idx);
			}
		});
	}

	fn scalar_convert_to_pooled(&mut self, sc: &Scalar) -> Scalar {
		if let Scalar::String(s) = sc {
			if let Some(hs) = s.as_heap_str() {
				// convert heap-stored string into pooled
				let idx = self.string_pool_add(hs);
				return Scalar::String(s.to_indexed(idx));
			}
		}

		sc.clone()
	}

	fn scalar_pool_delete_strings(&mut self, start: usize, end: usize) {
		for i in start..end {
			if let Scalar::String(s) = &self.scalar_pool[i] {
				if let Some(idx) = s.idx() {
					self.string_pool_remove(idx);
				}
			}
		}
	}

	fn scalar_pool_extend(&mut self, ss: &[Scalar]) -> usize {
		let idx = self.scalar_pool.len();

		for s in ss {
			let ns = self.scalar_convert_to_pooled(&s);
			self.scalar_pool.push(ns);
		}

		idx
	}

	fn scalar_pool_remove(&mut self, start: usize, end: usize) {
		let size = end - start;

		self.scalar_pool_delete_strings(start, end);
		self.scalar_pool.drain(start..end);

		self.scalar_1_idxs.iter_mut().for_each(|(pool_start, pool_end)| {
			if *pool_start >= size && *pool_start > start {
				*pool_start -= size;
				*pool_end -= size;
			}
		});
		self.scalar_2_idxs.iter_mut().for_each(|(pool_start, pool_end)| {
			if *pool_start >= size && *pool_start > start {
				*pool_start -= size;
				*pool_end -= size;
			}
		});
	}

	fn scalar_pool_overwrite(&mut self, start: usize, ss: &[Scalar]) {
		self.scalar_pool_delete_strings(start, start + ss.len());

		let mut i = start;
		for s in ss {
			let ns = self.scalar_convert_to_pooled(&s);
			self.scalar_pool[i] = ns;
			i += 1;
		}
	}

	// TODO: rewrite using scalar_pool_extend indeces
	fn scalar_pool_add(&mut self, insert_first: bool, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let start_1 = self.scalar_pool.len();
		let end_1 = start_1 + ss_1.len();
		let (start_2, end_2) = match ss_2.is_empty() {
			true => (0, 0),
			false => (end_1, end_1 + ss_2.len()),
		};

		self.scalar_pool_extend(ss_1);
		self.scalar_pool_extend(ss_2);

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
	// TODO: rewrite using scalar_pool_extend indeces
	fn scalar_pool_replace(&mut self, idx: usize, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let (pre_start_1, pre_end_1) = self.scalar_1_idxs[idx];
		let pre_size_1 = pre_end_1 - pre_start_1;

		// delete slot for first slice
		if ss_1.len() == pre_size_1 {
			// yay, new scalars fit in the existing slot
			self.scalar_pool_overwrite(pre_start_1, ss_1);
		} else {
			// we'll have to resize and extend :'(
			self.scalar_pool_remove(pre_start_1, pre_end_1);

			let start_1 = self.scalar_pool.len();
			let end_1 = start_1 + ss_1.len();

			self.scalar_pool_extend(ss_1);
			self.scalar_1_idxs[idx] = (start_1, end_1);
		}

		// delete slot for second slice, if present
		let (pre_start_2, pre_end_2) = self.scalar_2_idxs[idx];
		let pre_size_2 = pre_end_2 - pre_start_2;

		if ss_2.len() == pre_size_2 {
			self.scalar_pool_overwrite(pre_start_2, ss_2);
		} else {
			// we'll have to resize and extend :'(
			self.scalar_pool_remove(pre_start_2, pre_end_2);

			let (start_2, end_2) = if ss_2.is_empty() {
				(0, 0)
			} else {
				let s = self.scalar_pool.len();
				(s, s + ss_2.len())
			};

			self.scalar_pool_extend(ss_2);
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
		let key_idx = self.string_pool_add(key);
		match is_key_priority(key) {
			true => {
				// priority keys are inserted first
				self.scalar_pool_add(true, ss_1, ss_2);
				self.key_idxs.insert(0, key_idx);
			}
			false => {
				// insert new key last
				self.scalar_pool_add(false, ss_1, ss_2);
				self.key_idxs.push(key_idx);
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

impl AttributeStringSeek for Map {
	fn str_seek<'f>(&'f self, idx: usize) -> &'f str {
		if idx >= self.string_pool.len() {
			panic!("invalid pooled string #{idx} for Map");
		}
		let (start, end) = self.string_idxs[idx];
		&self.string_pool[start..end]
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
			write!(f, "{spacer}{key}=", spacer = if first { "" } else { " " })?;
			val.write_str(f, self)?;
			first = false;
		}

		Ok(())
	}
}

/* ----------------------- Tests ----------------------- */

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
	fn string_pool_behavior() {
		let mut attr = Map::new();

		attr.insert(
			"key_a",
			Value::from(&[
				Scalar::from(String::from("string #1")),
				Scalar::from(String::from("then string #2")),
				Scalar::from(String::from("finally string #3")),
			]),
		);
		attr.insert(
			"key_b",
			Value::from((
				&[
					Scalar::from(String::from("sub key 1")),
					Scalar::from(String::from("sub key #2")),
					Scalar::from(String::from("and sub key 3")),
				],
				&[Scalar::from(String::from("lala")), Scalar::from(String::from("lelele")), Scalar::from(String::from("lolololo"))],
			)),
		);
		attr.insert("key_c", Value::from("static_string"));
		attr.insert("key_d", Value::from(String::from("heap string")));

		assert_eq!(
			attr.to_string(),
			"key_a=[\"string #1\", \"then string #2\", \"finally string #3\"] key_b={\"sub key 1\": \"lala\", \"sub key #2\": \"lelele\", \"and sub key 3\": \"lolololo\"} key_c=\"static_string\" key_d=\"heap string\"",
		);
		assert_eq!(
			attr.string_pool,
			"key_astring #1then string #2finally string #3key_bsub key 1sub key #2and sub key 3lalalelelelolololokey_ckey_dheap string"
		);

		// replace pooled string list with a static list of the same size.
		attr.insert("key_a", Value::from(&[Scalar::from(123), Scalar::from("boo"), Scalar::from(456)]));
		assert_eq!(
			attr.to_string(),
			"key_a=[123, \"boo\", 456] key_b={\"sub key 1\": \"lala\", \"sub key #2\": \"lelele\", \"and sub key 3\": \"lolololo\"} key_c=\"static_string\" key_d=\"heap string\""
		);
		assert_eq!(attr.string_pool, "key_akey_bsub key 1sub key #2and sub key 3lalalelelelolololokey_ckey_dheap string");

		// replace string list with different size map
		attr.insert(
			"key_a",
			Value::from((
				&[Scalar::from(String::from("new sub key 1")), Scalar::from(String::from("with sub key 2"))],
				&[Scalar::from(3.14159), Scalar::from("i'm static!")],
			)),
		);
		assert_eq!(
			attr.to_string(),
			"key_a={\"new sub key 1\": 3.14159, \"with sub key 2\": \"i'm static!\"} key_b={\"sub key 1\": \"lala\", \"sub key #2\": \"lelele\", \"and sub key 3\": \"lolololo\"} key_c=\"static_string\" key_d=\"heap string\""
		);
		assert_eq!(
			attr.string_pool,
			"key_akey_bsub key 1sub key #2and sub key 3lalalelelelolololokey_ckey_dheap stringnew sub key 1with sub key 2"
		);

		// replace map with different sized list
		attr.insert("key_b", Value::from(&[Scalar::from(String::from("new string")), Scalar::from(12345)]));
		assert_eq!(
			attr.to_string(),
			"key_a={\"new sub key 1\": 3.14159, \"with sub key 2\": \"i'm static!\"} key_b=[\"new string\", 12345] key_c=\"static_string\" key_d=\"heap string\""
		);
		assert_eq!(attr.string_pool, "key_akey_bkey_ckey_dheap stringnew sub key 1with sub key 2new string");

		attr.insert("key_a", Value::from(1111));
		attr.insert("key_b", Value::from(2222));
		assert_eq!(attr.to_string(), "key_a=1111 key_b=2222 key_c=\"static_string\" key_d=\"heap string\"");
		assert_eq!(attr.string_pool, "key_akey_bkey_ckey_dheap string",);
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
