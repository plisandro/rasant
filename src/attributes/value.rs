use std::fmt;
use std::fmt::Write;

use crate::attributes::scalar::Scalar;

/// Value definition for all log operations.
/// These are associated with a single [`&str`] key in attribute maps for [Logger][`crate::Logger`]s.
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'e> {
	/// A single [`Scalar`] value.
	Scalar(Scalar),
	/// An ordered set of [`Scalar`] values.
	List(&'e [Scalar]),
	/// A map-like ordered set of { [`Scalar`] -> [`Scalar`] } value tuples.
	Map(&'e [Scalar], &'e [Scalar]),
}

/* ----------------------- Value implementation ----------------------- */

impl<'i> fmt::Display for Value<'i> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			Self::Scalar(s) => write!(f, "{}", s),
			Self::List(ss) => {
				write!(f, "[")?;
				for i in 0..ss.len() {
					if i != 0 {
						write!(f, ", ")?;
					}
					write!(f, "{}", ss[i])?;
				}
				write!(f, "]")
			}
			Self::Map(keys, ss) => {
				write!(f, "{{")?;
				for i in 0..keys.len() {
					if i != 0 {
						write!(f, ", ")?;
					}
					write!(f, "{key}: {val}", key = keys[i], val = ss[i])?;
				}
				write!(f, "}}")
			}
		}
	}
}

// Trait for known types/structs which can be casted into a [`Value`].
impl<'i> Value<'i> {
	/// Serializes a [`Value`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_string(&self, out: &mut String) {
		out.clear();
		write!(*out, "{}", self).expect("failed to serialize Value into_string()");
	}
}

/* ----------------------- Casting ----------------------- */

// casters for Value::Scalar
impl<'i, T: Into<Scalar>> From<T> for Value<'i> {
	fn from(t: T) -> Self {
		Self::Scalar(t.into())
	}
}

// casters for Value::List
impl<'i> From<&'i [Scalar]> for Value<'i> {
	fn from(s: &'i [Scalar]) -> Self {
		Self::List(s)
	}
}

/*
impl<'i, const N: usize> From<[Scalar; N]> for Value<'i> {
	fn from(a: [Scalar; N]) -> Self {
		Self::List(a.as_slice())
	}
}
*/

impl<'i, const N: usize> From<&'i [Scalar; N]> for Value<'i> {
	fn from(l: &'i [Scalar; N]) -> Self {
		Self::List(l.as_slice())
	}
}

/*
// casters for List of ToScalar's.
impl<'i, T: ToScalar, const N: usize> ToValue for [T; N] {
	fn to_value(&self) -> Value<'_> {
		Value::List(self.map(|x| x.to_scalar()).as_slice())
	}
}
*/

// casters for Value::Map
impl<'i> From<(&'i [Scalar], &'i [Scalar])> for Value<'i> {
	fn from(m: (&'i [Scalar], &'i [Scalar])) -> Self {
		Self::Map(m.0, m.1)
	}
}

/*
impl<'i, const N: usize> From<([Scalar; N], [Scalar; N])> for Value<'i> {
	fn from(m: ([Scalar; N], [Scalar; N])) -> Self {
		Self::Map(m.0.as_slice(), m.1.as_slice())
	}
}
*/

impl<'i, const N: usize> From<(&'i [Scalar; N], &'i [Scalar; N])> for Value<'i> {
	fn from(m: (&'i [Scalar; N], &'i [Scalar; N])) -> Self {
		Self::Map(m.0.as_slice(), m.1.as_slice())
	}
}

impl<'i> From<[&'i [Scalar]; 2]> for Value<'i> {
	fn from(m: [&'i [Scalar]; 2]) -> Self {
		Self::Map(m[0], m[1])
	}
}

/*
impl<'i, const N: usize> From<[[Scalar; N]; 2]> for Value<'i> {
	fn from(m: [[Scalar; N]; 2]) -> Self {
		Self::Map(m[0].as_slice(), m[1].as_slice())
	}
}
*/

impl<'i, const N: usize> From<[&'i [Scalar; N]; 2]> for Value<'i> {
	fn from(m: [&'i [Scalar; N]; 2]) -> Self {
		Self::Map(m[0].as_slice(), m[1].as_slice())
	}
}

impl<'i, const N: usize> From<&'i [&'i [Scalar; N]; 2]> for Value<'i> {
	fn from(m: &'i [&'i [Scalar; N]; 2]) -> Self {
		Self::Map(m[0].as_slice(), m[1].as_slice())
	}
}

/* ----------------------- Tests ----------------------- */

// TODO: add tests for Map
#[cfg(test)]
mod tests {
	use super::*;

	use crate::types::AttributeString;
	use ntime::{Duration, Timestamp};

	#[test]
	fn from_value_scalar() {
		let short_string = "lalala";
		let long_string = "this is a rather long string, which may be complicated";

		assert_eq!(Value::from(true), Value::Scalar(Scalar::Bool(true)));
		assert_eq!(Value::from(short_string), Value::Scalar(Scalar::String(AttributeString::from(short_string))));
		assert_eq!(Value::from(String::from(short_string)), Value::Scalar(Scalar::String(AttributeString::from(short_string))));
		assert_eq!(Value::from(long_string), Value::Scalar(Scalar::String(long_string.into())));
		assert_eq!(Value::from(String::from(long_string)), Value::Scalar(Scalar::String(long_string.into())));
		assert_eq!(Value::from(-12 as i8), Value::Scalar(Scalar::Int(-12)));
		assert_eq!(Value::from(345 as i16), Value::Scalar(Scalar::Int(345)));
		assert_eq!(Value::from(-678 as i32), Value::Scalar(Scalar::Int(-678)));
		assert_eq!(Value::from(9012 as i64), Value::Scalar(Scalar::Int(9012)));
		assert_eq!(Value::from(-3456 as i128), Value::Scalar(Scalar::LongInt(-3456)));
		assert_eq!(Value::from(7890 as isize), Value::Scalar(Scalar::Size(7890)));
		assert_eq!(Value::from(12 as u8), Value::Scalar(Scalar::Uint(12)));
		assert_eq!(Value::from(345 as u16), Value::Scalar(Scalar::Uint(345)));
		assert_eq!(Value::from(678 as u32), Value::Scalar(Scalar::Uint(678)));
		assert_eq!(Value::from(9012 as u64), Value::Scalar(Scalar::Uint(9012)));
		assert_eq!(Value::from(3456 as u128), Value::Scalar(Scalar::LongUint(3456)));
		assert_eq!(Value::from(7890 as usize), Value::Scalar(Scalar::Usize(7890)));
		// yaay precision!
		assert_eq!(Value::from(-123.456 as f32), Value::Scalar(Scalar::Float(-123.45600128173828)));
		assert_eq!(Value::from(789.012 as f64), Value::Scalar(Scalar::Float(789.012)));
		assert_eq!(Value::from(Duration::from_millis(12345)), Value::Scalar(Scalar::Uint(12)));
		assert_eq!(Value::from(&Duration::from_millis(67890)), Value::Scalar(Scalar::Uint(67)));
		assert_eq!(Value::from(Timestamp::from_millis(12345)), Value::Scalar(Scalar::Uint(12)));
		assert_eq!(Value::from(&Timestamp::from_millis(67890)), Value::Scalar(Scalar::Uint(67)));
	}

	// TODO: clean me up
	#[test]
	fn from_value_set() {
		let arr = [Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)];
		let slice = &[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)];
		//assert_eq!(Value::from(arr), Value::List(&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)]));
		assert_eq!(Value::from(&arr), Value::List(&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)]));
		assert_eq!(Value::from(slice), Value::List(&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)]));
	}

	// TODO: clean me up
	#[test]
	fn from_value_map() {
		let arrays = [
			&[Scalar::String("key_a".into()), Scalar::String("key_b".into()), Scalar::String("key_c".into())],
			&[Scalar::Bool(false), Scalar::Int(-123), Scalar::Float(456.789)],
		];
		let slices = &[&[Scalar::String("key_c".into()), Scalar::String("key_d".into())], &[Scalar::Bool(true), Scalar::Int(456)]];

		assert_eq!(
			//Value::from(arrays),
			Value::from(&arrays),
			Value::Map(
				&[Scalar::String("key_a".into()), Scalar::String("key_b".into()), Scalar::String("key_c".into())],
				&[Scalar::Bool(false), Scalar::Int(-123), Scalar::Float(456.789)]
			)
		);
		assert_eq!(
			Value::from(slices),
			Value::Map(&[Scalar::String("key_c".into()), Scalar::String("key_d".into())], &[Scalar::Bool(true), Scalar::Int(456)])
		);
	}

	#[test]
	fn dbg_format() {
		assert_eq!(format!("{}", Value::Scalar(Scalar::Bool(true))), "true");
		assert_eq!(format!("{}", Value::Scalar(Scalar::String(AttributeString::from("boo")))), "\"boo\"");
		assert_eq!(format!("{}", Value::Scalar(Scalar::Size(-12345678901234567))), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Value::Scalar(Scalar::Uint(123456))), "123456");
		assert_eq!(
			format!(
				"{}",
				Value::List(&[
					Scalar::Bool(true),
					Scalar::String("boo".into()),
					Scalar::String(AttributeString::from("abcd 1234")),
					Scalar::Size(-12345678901234567),
					Scalar::Uint(123456),
				])
			),
			"[true, \"boo\", \"abcd 1234\", -0x2bdc545d6b4b87, 123456]"
		);
		assert_eq!(
			format!(
				"{}",
				Value::Map(
					&[Scalar::Int(123), Scalar::Int(456), Scalar::Int(-789)],
					&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-111)],
				)
			),
			"{123: true, 456: \"boo\", -789: -0x6f}",
		);
	}

	#[test]
	fn into_string() {
		let mut out = String::from("lolololo!");
		Value::Scalar(Scalar::LongInt(-12345678901234567)).into_string(&mut out);
		assert_eq!(out, "-0x2bdc545d6b4b87");
	}
}
