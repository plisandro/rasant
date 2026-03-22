use std::fmt;

use crate::time::{Duration, Timestamp};
use std::thread;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
	Bool(bool),
	String(String),
	Int(i64),
	LongInt(i128),
	Size(isize),
	Uint(u64),
	LongUint(u128),
	Usize(usize),
	Float(f64),
}

pub trait ToValue {
	fn to_value(&self) -> Value;
}

impl Value {
	pub fn from<T: ToValue>(v: T) -> Self {
		v.to_value()
	}
}

impl ToValue for bool {
	fn to_value(&self) -> Value {
		Value::Bool(*self)
	}
}

impl ToValue for String {
	fn to_value(&self) -> Value {
		Value::String(self.clone())
	}
}

impl ToValue for &str {
	fn to_value(&self) -> Value {
		Value::String((*self).into())
	}
}

impl ToValue for Duration {
	fn to_value(&self) -> Value {
		Value::Uint((*self).as_secs())
	}
}

impl ToValue for &Duration {
	fn to_value(&self) -> Value {
		Value::Uint((*self).as_secs())
	}
}

impl ToValue for Timestamp {
	fn to_value(&self) -> Value {
		Value::Uint((*self).as_secs())
	}
}

impl ToValue for &Timestamp {
	fn to_value(&self) -> Value {
		Value::Uint((*self).as_secs())
	}
}

impl ToValue for thread::ThreadId {
	fn to_value(&self) -> Value {
		Value::String(format!("{:?}", *self))
	}
}

impl ToValue for &thread::ThreadId {
	fn to_value(&self) -> Value {
		Value::String(format!("{:?}", *self))
	}
}

macro_rules! cast_signed_to_value {
	($t: ty) => {
		impl ToValue for $t {
			fn to_value(&self) -> Value {
				Value::Int(*self as i64)
			}
		}
	};
}

cast_signed_to_value!(i8);
cast_signed_to_value!(i16);
cast_signed_to_value!(i32);
cast_signed_to_value!(i64);

impl ToValue for i128 {
	fn to_value(&self) -> Value {
		Value::LongInt(*self)
	}
}

impl ToValue for isize {
	fn to_value(&self) -> Value {
		Value::Size(*self)
	}
}

macro_rules! cast_unsigned_to_value {
	($t: ty) => {
		impl ToValue for $t {
			fn to_value(&self) -> Value {
				Value::Uint(*self as u64)
			}
		}
	};
}

cast_unsigned_to_value!(u8);
cast_unsigned_to_value!(u16);
cast_unsigned_to_value!(u32);
cast_unsigned_to_value!(u64);

impl ToValue for u128 {
	fn to_value(&self) -> Value {
		Value::LongUint(*self)
	}
}

impl ToValue for usize {
	fn to_value(&self) -> Value {
		Value::Usize(*self)
	}
}

macro_rules! cast_float_to_value {
	($t: ty) => {
		impl ToValue for $t {
			fn to_value(&self) -> Value {
				Value::Float(*self as f64)
			}
		}
	};
}

cast_float_to_value!(f32);
cast_float_to_value!(f64);

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let s = match &self {
			Self::Bool(b) => b.to_string(),
			Self::String(s) => s.to_string(),
			Self::Int(i) => i.to_string(),
			Self::LongInt(i) => {
				if *i < 1 {
					format!("-0x{:x}", -i)
				} else {
					format!("0x{:x}", i)
				}
			}
			Self::Size(s) => {
				if *s < 1 {
					format!("-0x{:x}", -s)
				} else {
					format!("0x{:x}", s)
				}
			}
			Self::Uint(i) => i.to_string(),
			Self::LongUint(i) => format!("0x{:x}", i),
			Self::Usize(u) => format!("0x{:x}", u),
			Self::Float(f) => f.to_string(),
		};
		write!(f, "{}", s)
	}
}

impl Value {
	pub fn to_quoted_string(&self) -> String {
		match &self {
			Self::String(s) => {
				let mut q = s.to_string();
				q.insert(0, '"');
				q.push('"');

				q
			}
			v => v.to_string(),
		}
	}

	pub fn to_json_string(&self) -> String {
		match &self {
			Self::Bool(b) => b.to_string(),
			Self::String(s) => {
				let mut q = s.to_string();
				q.insert(0, '"');
				q.push('"');

				q
			}
			Self::Int(i) => i.to_string(),
			Self::LongInt(i) => i.to_string(),
			Self::Size(s) => s.to_string(),
			Self::Uint(i) => i.to_string(),
			Self::LongUint(i) => i.to_string(),
			Self::Usize(u) => u.to_string(),
			Self::Float(f) => format!("{0:e}", f),
		}
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn type_to_value() {
		assert_eq!(true.to_value(), Value::Bool(true));
		assert_eq!("lalala".to_value(), Value::String("lalala".into()));
		assert_eq!(String::from("lololo").to_value(), Value::String("lololo".into()));
		assert_eq!((-12 as i8).to_value(), Value::Int(-12));
		assert_eq!((345 as i16).to_value(), Value::Int(345));
		assert_eq!((-678 as i32).to_value(), Value::Int(-678));
		assert_eq!((9012 as i64).to_value(), Value::Int(9012));
		assert_eq!((-3456 as i128).to_value(), Value::LongInt(-3456));
		assert_eq!((7890 as isize).to_value(), Value::Size(7890));
		assert_eq!((12 as u8).to_value(), Value::Uint(12));
		assert_eq!((345 as u16).to_value(), Value::Uint(345));
		assert_eq!((678 as u32).to_value(), Value::Uint(678));
		assert_eq!((9012 as u64).to_value(), Value::Uint(9012));
		assert_eq!((3456 as u128).to_value(), Value::LongUint(3456));
		assert_eq!((7890 as usize).to_value(), Value::Usize(7890));
		assert_eq!(
			// yaay precision!
			(-123.456 as f32).to_value(),
			Value::Float(-123.45600128173828)
		);
		assert_eq!((789.012 as f64).to_value(), Value::Float(789.012));
		assert_eq!(Duration::from_millis(12345).to_value(), Value::Uint(12));
		assert_eq!((&Duration::from_millis(67890)).to_value(), Value::Uint(67));
		assert_eq!(Timestamp::from_millis(12345).to_value(), Value::Uint(12));
		assert_eq!((&Timestamp::from_millis(67890)).to_value(), Value::Uint(67));
	}

	#[test]
	fn value_to_string() {
		for tc in [
			(Value::Bool(true), "true"),
			(Value::Bool(false), "false"),
			(Value::String("".into()), ""),
			(Value::String("abcd 1234".into()), "abcd 1234"),
			(Value::Int(-123), "-123"),
			(Value::Int(456), "456"),
			(Value::LongInt(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Value::LongInt(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Size(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Value::Size(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Uint(123456), "123456"),
			(Value::LongUint(12345678901234567), "0x2bdc545d6b4b87"),
			(Value::Usize(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Float(-1.2345), "-1.2345"),
			(Value::Float(6.78901), "6.78901"),
		] {
			let (v, want): (Value, &str) = tc;

			assert_eq!(v.to_string(), String::from(want));
		}
	}

	#[test]
	fn value_to_quoted_string() {
		for tc in [
			(Value::Bool(true), "true"),
			(Value::String("".into()), "\"\""),
			(Value::String("abcd 1234".into()), "\"abcd 1234\""),
			(Value::Int(-123), "-123"),
			(Value::LongInt(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Value::Size(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Uint(123456), "123456"),
			(Value::LongUint(12345678901234567), "0x2bdc545d6b4b87"),
			(Value::Usize(89801234567890123), "0x13f09bf3ecf84cb"),
			(Value::Float(-1.2345), "-1.2345"),
		] {
			let (v, want): (Value, &str) = tc;

			assert_eq!(v.to_quoted_string(), String::from(want));
		}
	}

	#[test]
	fn value_to_json_string() {
		for tc in [
			(Value::Bool(true), "true"),
			(Value::String("".into()), "\"\""),
			(Value::String("abcd 1234".into()), "\"abcd 1234\""),
			(Value::Int(-123), "-123"),
			(Value::LongInt(-12345678901234567), "-12345678901234567"),
			(Value::LongInt(89801234567890123), "89801234567890123"),
			(Value::Size(-12345678901234567), "-12345678901234567"),
			(Value::Size(89801234567890123), "89801234567890123"),
			(Value::Uint(123456), "123456"),
			(Value::LongUint(12345678901234567), "12345678901234567"),
			(Value::Usize(89801234567890123), "89801234567890123"),
			(Value::Float(-1234.56789012345), "-1.23456789012345e3"),
			(Value::Float(5678901.2345), "5.6789012345e6"),
		] {
			let (v, want): (Value, &str) = tc;

			assert_eq!(v.to_json_string(), String::from(want));
		}
	}
}
