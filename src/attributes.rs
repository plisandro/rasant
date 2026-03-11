pub mod value;

use std::collections::HashMap;
use std::fmt;

use crate::attributes::value::{ToValue, Value};

pub const KEY_ERROR: &str = "error";
pub const KEY_LEVEL: &str = "level";
pub const KEY_MESSAGE: &str = "message";
pub const KEY_TIME: &str = "time";
pub const KEY_TIMESTAMP: &str = "timestamp";
pub const PRIORITY_KEYS: [&str; 2] = [KEY_MESSAGE, KEY_ERROR];
pub const RESTRICTED_KEYS: [&str; 3] = [KEY_LEVEL, KEY_TIME, KEY_TIMESTAMP];

#[derive(Debug)]
pub struct Map {
	data: HashMap<String, Value>,
	keys: Vec<String>,
}

impl Clone for Map {
	fn clone(&self) -> Self {
		return Map {
			data: self.data.clone(),
			keys: self.keys.clone(),
		};
	}
}

impl Map {
	pub fn new() -> Self {
		Self {
			data: HashMap::new(),
			keys: Vec::new(),
		}
	}

	pub fn len(&self) -> usize {
		self.keys.len()
	}

	pub fn keys(&self) -> &Vec<String> {
		&self.keys
	}

	pub fn has(&self, key: &str) -> bool {
		self.get(key).is_some()
	}

	pub fn get(&self, key: &str) -> Option<&Value> {
		self.data.get(key)
	}

	pub fn insert_val(&mut self, key: &str, v: Value) {
		if key.len() == 0 {
			panic!("empty log attribute key {{\"\" -> {val}}}", val = v.to_string());
		}
		if key.chars().any(|c| c.is_whitespace()) {
			panic!("invalid log attribute key {{\"{key}\" -> {val}}}", val = v.to_string());
		}
		if RESTRICTED_KEYS.iter().find(|&&pk| pk == key).is_some() {
			panic!("cannot use restricted log attribute key {{\"{key}\" -> {val}}}", val = v.to_string());
		}

		if !self.data.contains_key(key.into()) {
			match PRIORITY_KEYS.iter().find(|&&pk| pk == key) {
				// priority keys are always returned first
				Some(_) => self.keys.insert(0, key.into()),
				None => self.keys.push(key.into()),
			}
		}
		/*
		if !self.data.contains_key(key.into()) {
			self.keys.push(key.into());
			self.keys.sort_by(|a, b| -> Ordering {
				// errors should always be listed first
				if a == ERROR_ATTRIBUTE {
					return Ordering::Less;
				}
				if b == ERROR_ATTRIBUTE {
					return Ordering::Greater;
				}
				return a.cmp(b);
			});
		}
		*/

		self.data.insert(key.into(), v);
	}

	pub fn insert<T: ToValue>(&mut self, key: &str, raw: T) {
		self.insert_val(key, raw.to_value());
	}

	pub fn get_as_string(&self, key: &str) -> String {
		match self.data.get(key) {
			Some(v) => v.to_string().clone(),
			None => "".into(),
		}
	}

	pub fn get_as_quoted_string(&self, key: &str) -> String {
		match self.data.get(key) {
			Some(v) => v.to_quoted_string().clone(),
			None => "".into(),
		}
	}

	pub fn get_as_json_string(&self, key: &str) -> String {
		match self.data.get(key) {
			Some(v) => v.to_json_string().clone(),
			None => "".into(),
		}
	}
}

impl fmt::Display for Map {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut out = String::new();
		let mut first = true;
		for k in self.keys() {
			if !first {
				out += " ";
			} else {
				first = false;
			}
			out += k.as_str();
			out += "=";
			out += self.get_as_quoted_string(k.as_str()).as_str();
		}

		write!(f, "{}", out)
	}
}

/* ----------------------- Tests ----------------------- */

#[test]
fn map_basic_operations() {
	let mut map = Map::new();

	assert_eq!(map.len(), 0);
	assert_eq!(map.keys(), &Vec::<String>::new());

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
fn map_empty_key() {
	let mut map = Map::new();
	map.insert("", "oh no");
}

#[test]
#[should_panic]
fn map_invalid_key() {
	let mut map = Map::new();
	map.insert("no\twhitespace\tin\tkeys", "please!");
}

#[test]
#[should_panic]
fn map_restricted_key() {
	let mut map = Map::new();
	map.insert("level", 55555);
}
