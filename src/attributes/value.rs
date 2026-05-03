use std::fmt;
use std::fmt::Write;

use crate::attributes::scalar::{Scalar, ToScalar};

/// Value definition for all log operations.
/// These are associated with a single [`&str`] key in attribute maps for [Logger][`crate::Logger`]s.
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'e> {
	/// A single [`Scalar`] value.
	Scalar(Scalar),
	/// An ordered set of [`Scalar`] values.
	Set(&'e [Scalar]),
	/// A map-like ordered set of { [`Scalar`] -> [`Scalar`] } value tuples.
	Map(&'e [Scalar], &'e [Scalar]),
}

/* ----------------------- Value implementation ----------------------- */

impl<'i> fmt::Display for Value<'i> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			Self::Scalar(s) => write!(f, "{}", s),
			Self::Set(ss) => {
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
	/// Creates a [`Value`] from a suitable type.
	pub fn from<T: ToValue>(v: &'i T) -> Value<'i> {
		v.to_value()
	}

	/// Serializes a [`Value`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_string(&self, out: &mut String) {
		out.clear();
		write!(*out, "{}", self).expect("failed to serialize Value into_string()");
	}
}

/* ----------------------- Casting helpers ----------------------- */

/// Trait for known types/structs which can be casted into a [`Value`].
pub trait ToValue {
	/// Casts the type to a [`Value`].
	fn to_value(&self) -> Value<'_>;
}

impl<'i> ToValue for Value<'i> {
	fn to_value(&self) -> Value<'_> {
		// TODO: fix me!
		self.clone()
	}
}

impl<'i, T: ToScalar> ToValue for T {
	fn to_value(&self) -> Value<'_> {
		Value::Scalar(self.to_scalar())
	}
}

// casters for List of Scalar's
impl<'i> ToValue for &'i [Scalar] {
	fn to_value(&self) -> Value<'_> {
		Value::Set(self)
	}
}

impl<'i, const N: usize> ToValue for [Scalar; N] {
	fn to_value(&self) -> Value<'_> {
		Value::Set(self.as_slice())
	}
}

// casters for Map of Scalar's.
impl<'i> ToValue for (&'i [Scalar], &'i [Scalar]) {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self.0, self.1)
	}
}

impl<'i, const N: usize> ToValue for ([Scalar; N], [Scalar; N]) {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self.0.as_slice(), self.1.as_slice())
	}
}

impl<'i, const N: usize> ToValue for (&[Scalar; N], &[Scalar; N]) {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self.0.as_slice(), self.1.as_slice())
	}
}

impl<'i> ToValue for [&'i [Scalar]; 2] {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self[0], self[1])
	}
}

impl<'i, const N: usize> ToValue for [[Scalar; N]; 2] {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self[0].as_slice(), self[1].as_slice())
	}
}

impl<'i, const N: usize> ToValue for [&[Scalar; N]; 2] {
	fn to_value(&self) -> Value<'_> {
		Value::Map(self[0].as_slice(), self[1].as_slice())
	}
}

/*/
// casters for List of ToScalar's.
impl<'i, T: ToScalar, const N: usize> ToValue for [T; N] {
	fn to_value(&self) -> Value<'_> {
		Value::Set(self.map(|x| x.to_scalar()).as_slice())
	}
}
*/

/* ----------------------- Tests ----------------------- */

// TODO: add tests for Map
#[cfg(test)]
mod tests {
	use super::*;

	use crate::types;
	use ntime::{Duration, Timestamp};

	#[test]
	fn to_value_scalar() {
		let short_string = "lalala";
		let long_string = "this is a rather long string, which may be complicated";

		assert_eq!(true.to_value(), Value::Scalar(Scalar::Bool(true)));
		assert_eq!(
			short_string.to_value(),
			Value::Scalar(Scalar::ShortString(types::ShortString::from(short_string).expect("ShortString serialization failed")))
		);
		assert_eq!(
			String::from(short_string).to_value(),
			Value::Scalar(Scalar::ShortString(types::ShortString::from(short_string).expect("ShortString serialization failed")))
		);
		assert_eq!(long_string.to_value(), Value::Scalar(Scalar::String(long_string.into())));
		assert_eq!(String::from(long_string).to_value(), Value::Scalar(Scalar::String(long_string.into())));
		assert_eq!((-12 as i8).to_value(), Value::Scalar(Scalar::Int(-12)));
		assert_eq!((345 as i16).to_value(), Value::Scalar(Scalar::Int(345)));
		assert_eq!((-678 as i32).to_value(), Value::Scalar(Scalar::Int(-678)));
		assert_eq!((9012 as i64).to_value(), Value::Scalar(Scalar::Int(9012)));
		assert_eq!((-3456 as i128).to_value(), Value::Scalar(Scalar::LongInt(-3456)));
		assert_eq!((7890 as isize).to_value(), Value::Scalar(Scalar::Size(7890)));
		assert_eq!((12 as u8).to_value(), Value::Scalar(Scalar::Uint(12)));
		assert_eq!((345 as u16).to_value(), Value::Scalar(Scalar::Uint(345)));
		assert_eq!((678 as u32).to_value(), Value::Scalar(Scalar::Uint(678)));
		assert_eq!((9012 as u64).to_value(), Value::Scalar(Scalar::Uint(9012)));
		assert_eq!((3456 as u128).to_value(), Value::Scalar(Scalar::LongUint(3456)));
		assert_eq!((7890 as usize).to_value(), Value::Scalar(Scalar::Usize(7890)));
		assert_eq!(
			// yaay precision!
			(-123.456 as f32).to_value(),
			Value::Scalar(Scalar::Float(-123.45600128173828))
		);
		assert_eq!((789.012 as f64).to_value(), Value::Scalar(Scalar::Float(789.012)));
		assert_eq!(Duration::from_millis(12345).to_value(), Value::Scalar(Scalar::Uint(12)));
		assert_eq!((&Duration::from_millis(67890)).to_value(), Value::Scalar(Scalar::Uint(67)));
		assert_eq!(Timestamp::from_millis(12345).to_value(), Value::Scalar(Scalar::Uint(12)));
		assert_eq!((&Timestamp::from_millis(67890)).to_value(), Value::Scalar(Scalar::Uint(67)));
	}

	#[test]
	fn to_value_set() {
		let arr = [Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)];
		let slice = &[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)];
		assert_eq!(arr.to_value(), Value::Set(&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)]));
		assert_eq!(slice.to_value(), Value::Set(&[Scalar::Bool(true), Scalar::String("boo".into()), Scalar::Size(-12345678901234567)]));
	}

	#[test]
	fn to_value_map() {
		let arrays = [
			[Scalar::String("key_a".into()), Scalar::String("key_b".into()), Scalar::String("key_c".into())],
			[Scalar::Bool(false), Scalar::Int(-123), Scalar::Float(456.789)],
		];
		let slices = &[&[Scalar::String("key_c".into()), Scalar::String("key_d".into())], &[Scalar::Bool(true), Scalar::Int(456)]];

		assert_eq!(
			arrays.to_value(),
			Value::Map(
				&[Scalar::String("key_a".into()), Scalar::String("key_b".into()), Scalar::String("key_c".into())],
				&[Scalar::Bool(false), Scalar::Int(-123), Scalar::Float(456.789)]
			)
		);
		assert_eq!(
			slices.to_value(),
			Value::Map(&[Scalar::String("key_c".into()), Scalar::String("key_d".into())], &[Scalar::Bool(true), Scalar::Int(456)])
		);
	}

	#[test]
	fn dbg_format() {
		assert_eq!(format!("{}", Value::Scalar(Scalar::Bool(true))), "true");
		assert_eq!(format!("{}", Value::Scalar(Scalar::String("boo".into()))), "\"boo\"");
		assert_eq!(format!("{}", Value::Scalar(Scalar::ShortString(types::ShortString::from("abcd 1234").unwrap()))), "\"abcd 1234\"");
		assert_eq!(format!("{}", Value::Scalar(Scalar::Size(-12345678901234567))), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Value::Scalar(Scalar::Uint(123456))), "123456");
		assert_eq!(
			format!(
				"{}",
				Value::Set(&[
					Scalar::Bool(true),
					Scalar::String("boo".into()),
					Scalar::ShortString(types::ShortString::from("abcd 1234").unwrap()),
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
