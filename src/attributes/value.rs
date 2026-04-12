use ntime::{Duration, Timestamp};
use std::fmt;
use std::thread;

/// Attribute value definition for all log operations.
/// These are associated with a single [`&str`] key in attribute maps for [logger][`crate::Logger`]s.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
	/// A [`bool`]ean.
	Bool(bool),
	/// An owned [`String`].
	String(String),
	/// An integer, internally stored as a [`i64`].
	Int(i64),
	/// A long integer, internally stored as a [`i128`].
	LongInt(i128),
	/// A pointer-sized integer, stored as a [`isize`].
	Size(isize),
	/// An unsigned integer, internally stored as a [`u64`].
	Uint(u64),
	/// An unsigned long integer, internally stored as a [`i128`].
	LongUint(u128),
	/// A pointer-sized unsigned integer, stored as a [`isize`].
	Usize(usize),
	/// A float, internally stored as a [`f64`].
	Float(f64),
}

/* ----------------------- Casting helpers ----------------------- */

/// Trait for known types/structs which can be casted into a [`Value`].
pub trait ToValue {
	/// Casts the type to a [`Value`].
	fn to_value(&self) -> Value;
}

impl Value {
	/// Yields the underlying type associated with a given [`Value`].
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

/* ----------------------- Value implementation ----------------------- */

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			Self::Bool(b) => write!(f, "{}", b),
			Self::String(s) => write!(f, "\"{}\"", s),
			Self::Int(i) => write!(f, "{}", i),
			Self::LongInt(i) => {
				if *i < 1 {
					write!(f, "-0x{:x}", -i)
				} else {
					write!(f, "0x{:x}", i)
				}
			}
			Self::Size(s) => {
				if *s < 1 {
					write!(f, "-0x{:x}", -s)
				} else {
					write!(f, "0x{:x}", s)
				}
			}
			Self::Uint(i) => write!(f, "{}", i),
			Self::LongUint(u) => write!(f, "0x{:x}", u),
			Self::Usize(u) => write!(f, "0x{:x}", u),
			Self::Float(fl) => write!(f, "{}", fl),
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
	fn dbg_format() {
		assert_eq!(format!("{}", Value::Bool(true)), "true");
		assert_eq!(format!("{}", Value::Bool(false)), "false");
		assert_eq!(format!("{}", Value::String("".into())), "\"\"");
		assert_eq!(format!("{}", Value::String("abcd 1234".into())), "\"abcd 1234\"");
		assert_eq!(format!("{}", Value::Int(-123)), "-123");
		assert_eq!(format!("{}", Value::Int(456)), "456");
		assert_eq!(format!("{}", Value::LongInt(-12345678901234567)), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Value::LongInt(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Value::Size(-12345678901234567)), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Value::Size(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Value::Uint(123456)), "123456");
		assert_eq!(format!("{}", Value::LongUint(12345678901234567)), "0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Value::Usize(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Value::Float(-1.2345)), "-1.2345");
		assert_eq!(format!("{}", Value::Float(6.78901)), "6.78901");
	}
}
