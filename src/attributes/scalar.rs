use ntime::{Duration, Timestamp};
use std::fmt;
use std::thread;

use crate::attributes::Map;
use crate::level::Level;
use crate::types::AttributeString;

/// [`Scalar`] definitions for all log operations.
/// Scalars are the basic data units for Rasant, representing a single type.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
	/// A [`bool`]ean.
	Bool(bool),
	/// An owned [`AttributeString`].
	String(AttributeString),
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

/* ----------------------- Implementation ----------------------- */

impl<'i> Scalar {
	/// Writes a string representation of a [`Scalar`] into an [`fmt::Write`].
	pub fn write_str<T: fmt::Write>(&self, out: &mut T, attrs: &Map) -> fmt::Result {
		match &self {
			Self::Bool(b) => write!(out, "{}", b),
			Self::String(s) => write!(out, "\"{}\"", s.as_str(attrs)),
			Self::Int(i) => write!(out, "{}", i),
			Self::LongInt(i) => {
				if *i < 1 {
					write!(out, "-0x{:x}", -i)
				} else {
					write!(out, "0x{:x}", i)
				}
			}
			Self::Size(s) => {
				if *s < 1 {
					write!(out, "-0x{:x}", -s)
				} else {
					write!(out, "0x{:x}", s)
				}
			}
			Self::Uint(i) => write!(out, "{}", i),
			Self::LongUint(u) => write!(out, "0x{:x}", u),
			Self::Usize(u) => write!(out, "0x{:x}", u),
			Self::Float(fl) => write!(out, "{}", fl),
		}
	}

	/// Creates an array of [`Scalar`]s from a suitable type.
	pub fn to_array<const N: usize, T: ToScalarArray<'i, N>>(v: T) -> [Self; N] {
		v.to_scalar_array()
	}

	/// Serializes a [`Scalar`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_string(&self, out: &mut String, attrs: &Map) {
		out.clear();
		self.write_str(out, attrs).expect("failed to serialize Scalar into_string()");
	}
}

/* ----------------------- Casting ----------------------- */

impl From<bool> for Scalar {
	fn from(b: bool) -> Self {
		Self::Bool(b)
	}
}

impl From<String> for Scalar {
	fn from(s: String) -> Self {
		Self::String(AttributeString::from(s))
	}
}

impl From<&'static str> for Scalar {
	fn from(s: &'static str) -> Self {
		Self::String(AttributeString::from(s))
	}
}

impl From<Duration> for Scalar {
	fn from(d: Duration) -> Self {
		Self::Uint(d.as_secs())
	}
}

impl From<&Duration> for Scalar {
	fn from(d: &Duration) -> Self {
		Self::Uint(d.as_secs())
	}
}

impl From<Timestamp> for Scalar {
	fn from(t: Timestamp) -> Self {
		Self::Uint(t.as_secs())
	}
}

impl From<&Timestamp> for Scalar {
	fn from(t: &Timestamp) -> Self {
		Self::Uint(t.as_secs())
	}
}

impl From<thread::ThreadId> for Scalar {
	fn from(t: thread::ThreadId) -> Self {
		Self::String(AttributeString::from(format!("{:?}", t)))
	}
}

impl From<&thread::ThreadId> for Scalar {
	fn from(t: &thread::ThreadId) -> Self {
		Self::String(AttributeString::from(format!("{:?}", t)))
	}
}

impl From<Level> for Scalar {
	fn from(t: Level) -> Self {
		Self::String(AttributeString::from(t.to_string()))
	}
}

impl From<&Level> for Scalar {
	fn from(t: &Level) -> Self {
		Self::String(AttributeString::from(t.to_string()))
	}
}

macro_rules! cast_signed_to_scalar {
	($t: ty) => {
		impl From<$t> for Scalar {
			fn from(x: $t) -> Self {
				Self::Int(x as i64)
			}
		}
	};
}

cast_signed_to_scalar!(i8);
cast_signed_to_scalar!(i16);
cast_signed_to_scalar!(i32);
cast_signed_to_scalar!(i64);

impl From<i128> for Scalar {
	fn from(x: i128) -> Self {
		Self::LongInt(x)
	}
}

impl From<isize> for Scalar {
	fn from(x: isize) -> Self {
		Self::Size(x)
	}
}

macro_rules! cast_unsigned_to_scalar {
	($t: ty) => {
		impl From<$t> for Scalar {
			fn from(x: $t) -> Self {
				Self::Uint(x as u64)
			}
		}
	};
}

cast_unsigned_to_scalar!(u8);
cast_unsigned_to_scalar!(u16);
cast_unsigned_to_scalar!(u32);
cast_unsigned_to_scalar!(u64);

impl From<u128> for Scalar {
	fn from(x: u128) -> Self {
		Self::LongUint(x)
	}
}

impl From<usize> for Scalar {
	fn from(x: usize) -> Self {
		Self::Usize(x)
	}
}

macro_rules! cast_float_to_scalar {
	($t: ty) => {
		impl From<$t> for Scalar {
			fn from(x: $t) -> Self {
				Self::Float(x as f64)
			}
		}
	};
}

cast_float_to_scalar!(f32);
cast_float_to_scalar!(f64);

/* ----------------------- Scalar slice helper implementation ----------------------- */

/// Trait for known types/structs which can be casted into an array of [`Scalar`].
pub trait ToScalarArray<'t, const N: usize> {
	/// Casts the type to a [`Scalar`] array.
	fn to_scalar_array(self) -> [Scalar; N];
}

impl<'i, T: Into<Scalar>> ToScalarArray<'i, 1> for T
where
	Scalar: From<T>,
{
	fn to_scalar_array(self) -> [Scalar; 1] {
		[Scalar::from(self)]
	}
}

impl<'i, T: Into<Scalar>, const N: usize> ToScalarArray<'i, N> for [T; N]
where
	Scalar: From<T>,
{
	fn to_scalar_array(self) -> [Scalar; N] {
		self.map(|x| Scalar::from(x))
	}
}

/*
impl<'i, T: ToScalar, const N: usize> ToScalarArray<'i, N> for &[T] {
	fn to_scalar_array(self) -> [Scalar; N] {
		let out: [Scalar; N] = array::from_fn(|i| self[i].to_scalar());
		out
	}
}
*/

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_scalar() {
		let short_string = "lalala";
		let long_string = "this is a rather long string, which may be complicated";

		assert_eq!(Scalar::from(true), Scalar::Bool(true));
		assert_eq!(Scalar::from(short_string), Scalar::String(AttributeString::from(short_string)));
		assert_eq!(Scalar::from(String::from(short_string)), Scalar::String(AttributeString::from(String::from(short_string))));
		assert_eq!(Scalar::from(long_string), Scalar::String(long_string.into()));
		assert_eq!(Scalar::from(String::from(long_string)), Scalar::String(AttributeString::from(String::from(long_string))));
		assert_eq!(Scalar::from(-12 as i8), Scalar::Int(-12));
		assert_eq!(Scalar::from(345 as i16), Scalar::Int(345));
		assert_eq!(Scalar::from(-678 as i32), Scalar::Int(-678));
		assert_eq!(Scalar::from(9012 as i64), Scalar::Int(9012));
		assert_eq!(Scalar::from(-3456 as i128), Scalar::LongInt(-3456));
		assert_eq!(Scalar::from(7890 as isize), Scalar::Size(7890));
		assert_eq!(Scalar::from(12 as u8), Scalar::Uint(12));
		assert_eq!(Scalar::from(345 as u16), Scalar::Uint(345));
		assert_eq!(Scalar::from(678 as u32), Scalar::Uint(678));
		assert_eq!(Scalar::from(9012 as u64), Scalar::Uint(9012));
		assert_eq!(Scalar::from(3456 as u128), Scalar::LongUint(3456));
		assert_eq!(Scalar::from(7890 as usize), Scalar::Usize(7890));
		// yaay precision!
		assert_eq!(Scalar::from(-123.456 as f32), Scalar::Float(-123.45600128173828));
		assert_eq!(Scalar::from(789.012 as f64), Scalar::Float(789.012));
		assert_eq!(Scalar::from(Duration::from_millis(12345)), Scalar::Uint(12));
		assert_eq!(Scalar::from(&Duration::from_millis(67890)), Scalar::Uint(67));
		assert_eq!(Scalar::from(Timestamp::from_millis(12345)), Scalar::Uint(12));
		assert_eq!(Scalar::from(&Timestamp::from_millis(67890)), Scalar::Uint(67));
	}

	#[test]
	fn into_string() {
		for tc in [
			(Scalar::Bool(true), "true"),
			(Scalar::Bool(false), "false"),
			(Scalar::String("".into()), "\"\""),
			(Scalar::String("abcd 1234".into()), "\"abcd 1234\""),
			(Scalar::String(String::from("heap String").into()), "\"heap String\""),
			(Scalar::Int(-123), "-123"),
			(Scalar::Int(456), "456"),
			(Scalar::LongInt(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Scalar::LongInt(89801234567890123), "0x13f09bf3ecf84cb"),
			(Scalar::Size(-12345678901234567), "-0x2bdc545d6b4b87"),
			(Scalar::Size(89801234567890123), "0x13f09bf3ecf84cb"),
			(Scalar::Uint(123456), "123456"),
			(Scalar::LongUint(12345678901234567), "0x2bdc545d6b4b87"),
			(Scalar::Usize(89801234567890123), "0x13f09bf3ecf84cb"),
			(Scalar::Float(-1.2345), "-1.2345"),
			(Scalar::Float(6.78901), "6.78901"),
		] {
			let (s, want): (Scalar, &str) = tc;

			let mut out = String::from("lalalala!");
			let attrs = Map::new();

			s.into_string(&mut out, &attrs);
			assert_eq!(out, want);
		}
	}
}
