pub mod scalar;
pub mod value;

use std::fmt;
use std::slice;

use crate::constant::{ATTRIBUTE_KEY_ERROR, ATTRIBUTE_KEYS_PRIORITY, ATTRIBUTE_KEYS_RESTRICTED};

pub use scalar::Scalar;
pub use value::Value;

/// Metadata flags for attributes.
pub enum MetadataField {
	/// The attribute key is restricted - i.e. it cannot be set by users.
	Restricted = (1 << 0),
	/// Priority attribute, which are always listed/returned first.
	Priority = (1 << 1),
	/// An attribute for an error.
	Error = (1 << 2),
	// TODO: add ephemeral flag
}

pub trait MetadataImpl {
	fn from_key<'f>(key: &'f str) -> Self;
	fn get(&self, field: MetadataField) -> bool;
	fn set(&mut self, field: MetadataField, value: bool);
}

/// Metadata associated with {key: attribute} pairs.
// usize is overkill for this usecase, but guarantees memory alignment within the vectors used by Map.
pub type Metadata = usize;

impl MetadataImpl for Metadata {
	fn from_key<'f>(key: &'f str) -> Self {
		let mut res: Metadata = 0;

		for restricted_key in ATTRIBUTE_KEYS_RESTRICTED {
			if key == restricted_key {
				// don't bother resolving the rest of metadata for restricted keys
				return MetadataField::Restricted as Metadata;
			}
		}

		for priority_key in ATTRIBUTE_KEYS_PRIORITY {
			if key == priority_key {
				res |= MetadataField::Priority as Metadata;
				break;
			}
		}

		if key == ATTRIBUTE_KEY_ERROR {
			res |= MetadataField::Error as Metadata;
		}

		return res;
	}

	fn get(&self, field: MetadataField) -> bool {
		self & (field as usize) != 0
	}

	fn set(&mut self, field: MetadataField, value: bool) {
		match value {
			true => *self |= field as Metadata,
			false => *self &= !(field as Metadata),
		}
	}
}

/// A store for a ordered map of (key -> [`Value`])
#[derive(Debug, Clone)]
pub struct Map {
	/// A container for all strings in this map (keys and scalars)
	string_pool: String,
	/// Indeces for every string in the pool: [start, end)
	string_idxs: Vec<(usize, usize)>,
	/// Name string index and metadata for all keys.
	keys: Vec<(usize, Metadata)>,
	/// A container for all Scalars used by Values in this store.
	scalar_pool: Vec<Scalar>,
	/// Indexes for 1st and 2nd set of Scalar's associated with each key, as [1_start, end), [2_start, 2_end)
	/// (0.0) indicates no 2nd set present.
	scalar_idxs: Vec<(usize, usize, usize, usize)>,
}

impl Map {
	pub fn new() -> Self {
		Self {
			string_pool: String::new(),
			string_idxs: Vec::new(),
			keys: Vec::new(),
			scalar_pool: Vec::new(),
			scalar_idxs: Vec::new(),
		}
	}

	pub fn clear(&mut self) {
		self.string_pool.clear();
		self.string_idxs.clear();
		self.keys.clear();
		self.scalar_pool.clear();
		self.scalar_idxs.clear();
	}

	pub fn copy_from(&mut self, other: &Self) {
		self.string_pool.clear();
		self.string_idxs.clear();
		self.keys.clear();
		self.scalar_pool.clear();
		self.scalar_idxs.clear();

		if !other.is_empty() {
			self.string_pool.push_str(&other.string_pool);
			self.string_idxs.extend(&other.string_idxs);
			self.keys.extend(&other.keys);
			// iterating over scalar_pool yields &Clone instead of Clone >:(
			self.scalar_pool.extend_from_slice(&other.scalar_pool);
			self.scalar_idxs.extend(&other.scalar_idxs);
		}
	}

	pub fn len(&self) -> usize {
		self.keys.len()
	}

	fn is_empty(&self) -> bool {
		self.keys.is_empty()
	}

	fn store_size(&self) -> usize {
		self.scalar_pool.len()
	}

	pub fn key_iter(&self) -> MapKeyIter<'_> {
		MapKeyIter::new(self)
	}

	pub fn key_value_iter(&self) -> MapKeyValueIter<'_> {
		MapKeyValueIter::new(self)
	}

	pub fn iter(&self) -> MapIter<'_> {
		MapIter::new(self)
	}

	// for key strings (relatively short, and small in number), a linear
	// search turns out to be the most efficient way to seek.
	fn idx_by_key(&self, key: &str) -> Option<usize> {
		let key_size = key.len();
		for i in 0..self.keys.len() {
			let (key_start, key_end) = self.string_idxs[self.keys[i].0];
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
		match idx < self.keys.len() {
			false => None,
			true => {
				let (start, end) = self.string_idxs[self.keys[idx].0];
				Some(&self.string_pool[start..end])
			}
		}
	}

	fn key_meta_by_idx(&self, idx: usize) -> Option<(&str, Metadata)> {
		match idx < self.keys.len() {
			false => None,
			true => {
				let (str_idx, meta) = self.keys[idx];
				let (start, end) = self.string_idxs[str_idx];
				Some((&self.string_pool[start..end], meta))
			}
		}
	}

	fn meta_by_idx(&self, idx: usize) -> Metadata {
		self.keys[idx].1
	}

	fn value_by_idx(&self, idx: usize) -> Value<'_> {
		let (start_1, end_1, start_2, end_2) = self.scalar_idxs[idx];

		match start_2 != 0 || end_2 != 0 {
			// a 2nd set of scalars means we have a Map
			true => Value::from((&self.scalar_pool[start_1..end_1], &self.scalar_pool[start_2..end_2])),
			false => Value::from(&self.scalar_pool[start_1..end_1]),
		}
	}

	pub fn has(&self, key: &str) -> bool {
		self.idx_by_key(key).is_some()
	}

	fn has_idx(&self, idx: usize) -> bool {
		idx < self.keys.len()
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
		self.keys.iter_mut().for_each(|(idx, _)| {
			if *idx > 0 && *idx >= del_idx {
				*idx -= 1;
			}
		});
		self.scalar_pool.iter_mut().for_each(|mut v| {
			if let Scalar::StringIndex(idx, _) = &mut v {
				if *idx != 0 && *idx >= del_idx {
					*idx -= 1
				}
			}
		});
	}

	fn scalar_convert_to_pooled(&mut self, sc: &Scalar) -> Scalar {
		if let Scalar::String(s, needs_escaping) = sc {
			let idx = self.string_pool_add(s);
			return Scalar::StringIndex(idx, *needs_escaping);
		}

		sc.clone()
	}

	fn scalar_pool_delete_strings(&mut self, start: usize, end: usize) {
		for i in start..end {
			if let Scalar::StringIndex(idx, _) = &self.scalar_pool[i] {
				self.string_pool_remove(*idx);
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

		self.scalar_idxs.iter_mut().for_each(|(pool_start_1, pool_end_1, pool_start_2, pool_end_2)| {
			if *pool_start_1 >= size && *pool_start_1 > start {
				*pool_start_1 -= size;
				*pool_end_1 -= size;
			}
			if *pool_start_2 >= size && *pool_start_2 > start {
				*pool_start_2 -= size;
				*pool_end_2 -= size;
			}
		})
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

	fn scalar_pool_add(&mut self, insert_first: bool, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let start_1 = self.scalar_pool_extend(ss_1);
		let end_1 = start_1 + ss_1.len();

		let (mut start_2, mut end_2) = (0 as usize, 0 as usize);
		if !ss_2.is_empty() {
			start_2 = self.scalar_pool_extend(ss_2);
			end_2 = start_2 + ss_2.len();
		};

		match insert_first {
			true => self.scalar_idxs.insert(0, (start_1, end_1, start_2, end_2)),
			false => self.scalar_idxs.push((start_1, end_1, start_2, end_2)),
		}
	}

	fn scalar_pool_replace(&mut self, idx: usize, ss_1: &[Scalar], ss_2: &[Scalar]) {
		let (mut start_1, mut end_1, _, _) = self.scalar_idxs[idx];
		let pre_size_1 = end_1 - start_1;

		// delete slot for first slice
		if ss_1.len() == pre_size_1 {
			// yay, new scalars fit in the existing slot
			self.scalar_pool_overwrite(start_1, ss_1);
		} else {
			// we'll have to resize and extend :'(
			self.scalar_pool_remove(start_1, end_1);

			start_1 = self.scalar_pool_extend(ss_1);
			end_1 = start_1 + ss_1.len();

			self.scalar_idxs[idx].0 = start_1;
			self.scalar_idxs[idx].1 = end_1;
		}

		// delete slot for second slice, if present
		let (_, _, mut start_2, mut end_2) = self.scalar_idxs[idx];
		let pre_size_2 = end_2 - start_2;

		if ss_2.len() == pre_size_2 {
			self.scalar_pool_overwrite(start_2, ss_2);
		} else {
			self.scalar_pool_remove(start_2, end_2);

			if ss_2.is_empty() {
				start_2 = 0;
				end_2 = 0;
			} else {
				start_2 = self.scalar_pool_extend(ss_2);
				end_2 = start_2 + ss_2.len();
			};

			self.scalar_idxs[idx].2 = start_2;
			self.scalar_idxs[idx].3 = end_2;
		}
	}

	pub fn get(&self, key: &str) -> Option<(Value<'_>, Metadata)> {
		match self.idx_by_key(key) {
			Some(i) => {
				let meta = self.meta_by_idx(i);
				Some((self.value_by_idx(i), meta))
			}
			None => None,
		}
	}

	pub fn get_value(&self, key: &str) -> Option<Value<'_>> {
		match self.idx_by_key(key) {
			Some(i) => Some(self.value_by_idx(i)),
			None => None,
		}
	}

	fn set(&mut self, key: &str, val: &Value) {
		if key.is_empty() {
			panic!("empty log attribute key {{\"\" -> {val:?}}}");
		}
		if key.chars().any(|c| c.is_whitespace() || !c.is_ascii()) {
			panic!("invalid log attribute key {{\"{key}\" -> {val:?}}}");
		}

		let meta = Metadata::from_key(key);
		if meta.get(MetadataField::Restricted) {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {val:?}}}");
		}

		let (ss_1, ss_2): (&[Scalar], &[Scalar]) = match val {
			Value::Scalar(s) => (slice::from_ref(s), &[]),
			Value::List(ss) => (*ss, &[]),
			Value::Map(keys, vals) => {
				if keys.len() != vals.len() {
					panic!("Map scalars mismatch for attribute key {{{key} -> {val:?}}}");
				}
				// TODO: handling duplicate keys without a panic would be nice...
				for i in 0..keys.len() {
					let key_i = &keys[i];
					for j in i + 1..keys.len() {
						let key_j = &keys[j];
						if key_i == key_j {
							panic!("Duplicate key for Map {{{key_i} -> {val_i}}} vs {{{key_j} -> {val_j}}}", val_i = &vals[i], val_j = &vals[j]);
						}
					}
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
		match meta.get(MetadataField::Priority) {
			true => {
				// priority keys are inserted first
				self.scalar_pool_add(true, ss_1, ss_2);
				self.keys.insert(0, (key_idx, meta));
			}
			false => {
				// insert new key last
				self.scalar_pool_add(false, ss_1, ss_2);
				self.keys.push((key_idx, meta));
			}
		}
	}

	pub fn insert_ref(&mut self, key: &str, val: &Value) {
		self.set(key, val);
	}

	pub fn insert(&mut self, key: &str, val: Value) {
		self.set(key, &val);
	}

	pub fn str_by_idx<'f>(&'f self, idx: usize) -> &'f str {
		if idx >= self.string_pool.len() {
			panic!("invalid pooled string #{idx} for Map");
		}

		let (start, end) = self.string_idxs[idx];
		&self.string_pool[start..end]
	}
}

/// A key iterator for attribute maps.
pub struct MapKeyIter<'s> {
	map: &'s Map,
	idx: usize,
}

impl<'i> MapKeyIter<'i> {
	/// Intiializes an attribute map key iterator.
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

/// A {key, [`Value`]} iterator for attribute maps.
pub struct MapKeyValueIter<'s> {
	map: &'s Map,
	idx: usize,
}

impl<'i> MapKeyValueIter<'i> {
	/// Intiializes an attribute map {key, [`Value`]} iterator.
	pub fn new(map: &'i Map) -> Self {
		Self { map: map, idx: 0 }
	}
}

impl<'i> Iterator for MapKeyValueIter<'i> {
	// {key: Value}
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

/// A {key, [`Value`], [`Metadata`]} iterator for attribute maps.
pub struct MapIter<'s> {
	map: &'s Map,
	idx: usize,
}

impl<'i> MapIter<'i> {
	/// Intiializes an attribute map key -> {key, [`Value`], [`Metadata`]} iterator.
	pub fn new(map: &'i Map) -> Self {
		Self { map: map, idx: 0 }
	}
}

impl<'i> Iterator for MapIter<'i> {
	// {key: Value}
	type Item = (&'i str, Value<'i>, Metadata);

	fn next(&mut self) -> Option<Self::Item> {
		match self.map.key_by_idx(self.idx) {
			None => None,
			Some(key) => {
				// TODO: clean me up
				let meta = self.map.keys[self.idx].1;
				let val = self.map.value_by_idx(self.idx);
				self.idx += 1;

				Some((key, val, meta))
			}
		}
	}
}

impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut first: bool = true;
		for (key, val, _) in self.iter() {
			write!(f, "{spacer}{key}=", spacer = if first { "" } else { " " })?;
			val.write_fmt(f, self)?;
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
		attr.set("a", &Value::Scalar(Scalar::from("lalala")));
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
			"key_a={\"new sub key 1\": 3.14159, \"with sub key 2\": \"i\\'m static!\"} key_b={\"sub key 1\": \"lala\", \"sub key #2\": \"lelele\", \"and sub key 3\": \"lolololo\"} key_c=\"static_string\" key_d=\"heap string\""
		);
		assert_eq!(
			attr.string_pool,
			"key_akey_bsub key 1sub key #2and sub key 3lalalelelelolololokey_ckey_dheap stringnew sub key 1with sub key 2"
		);

		// replace map with different sized list
		attr.insert("key_b", Value::from(&[Scalar::from(String::from("new string")), Scalar::from(12345)]));
		assert_eq!(
			attr.to_string(),
			"key_a={\"new sub key 1\": 3.14159, \"with sub key 2\": \"i\\'m static!\"} key_b=[\"new string\", 12345] key_c=\"static_string\" key_d=\"heap string\""
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
	fn insert_whitespaces_key() {
		Map::new().set("no whitespace\tin\tkeys", &Value::from("please!"));
	}

	#[test]
	#[should_panic]
	fn insert_non_ascii() {
		Map::new().set("como_estás", &Value::from(1234));
	}

	#[test]
	#[should_panic]
	fn insert_unicode_key() {
		Map::new().set("oh❤pretty!", &Value::from(5678));
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
	#[should_panic]
	fn insert_duplicate_key_map() {
		Map::new().set(
			"wrong_map",
			&Value::Map(
				[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_a")].as_slice(),
				[Scalar::from(123), Scalar::from(456.789), Scalar::from("oh no")].as_slice(),
			),
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

		assert_eq!(got_keys, &["error", "key_a", "key_b", "key_c", "key_d"]);
	}

	#[test]
	fn key_value_iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", Value::from(123));
		attr.insert("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)]));
		attr.insert("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)]));
		attr.insert("error", Value::from("an error"));
		attr.insert("key_d", Value::from(&[Scalar::from("new"), Scalar::from("key")]));

		let mut got: Vec<(&str, Value)> = Vec::new();
		for kvs in attr.key_value_iter() {
			got.push((kvs.0, kvs.1));
		}

		assert_eq!(
			got,
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
	fn full_iterator() {
		let mut attr = Map::new();

		attr.insert("key_a", Value::from(123));
		attr.insert("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)]));
		attr.insert("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)]));
		attr.insert("error", Value::from("an error"));
		attr.insert("key_d", Value::from(&[Scalar::from("new"), Scalar::from("key")]));

		let mut got: Vec<(&str, Value, Metadata)> = Vec::new();
		for kvs in attr.iter() {
			got.push(kvs);
		}

		assert_eq!(
			got,
			&[
				("error", Value::from("an error"), 0b110),
				("key_a", Value::from(123), 0),
				("key_b", Value::from(&[Scalar::from(456), Scalar::from(true)]), 0),
				("key_c", Value::from(&[Scalar::from(789), Scalar::from(false)]), 0),
				("key_d", Value::from(&[Scalar::from("new"), Scalar::from("key")]), 0),
			]
		);
	}
}
