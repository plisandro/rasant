use ntime::{Duration, Timestamp};
use std::fmt;
use std::fmt::Write;
use std::thread;

use crate::types;

/// [`Scalar`] definitions for all log operations.
/// Scalars are the basic data units for Rasant, representing a single type.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
	/// A [`bool`]ean.
	Bool(bool),
	/// An owned short string, akin to [`&str`].
	ShortString(types::ShortString),
	/// An owned [`String`], using heap storage.
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

/* ----------------------- Implementation ----------------------- */

impl fmt::Display for Scalar {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			Self::Bool(b) => write!(f, "{}", b),
			Self::ShortString(ss) => write!(f, "\"{}\"", ss.as_str()),
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

impl<'i> Scalar {
	/// Creates a [`Scalar`] from a suitable type.
	pub fn from<T: ToScalar>(v: T) -> Self {
		v.to_scalar()
	}

	/// Creates an array of [`Scalar`]s from a suitable type.
	pub fn to_array<const N: usize, T: ToScalarArray<'i, N>>(v: T) -> [Self; N] {
		v.to_scalar_array()
	}

	/// Serializes a [`Scalar`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_string(&self, out: &mut String) {
		out.clear();
		write!(*out, "{}", self).expect("failed to serialize Scalar into_string()");
	}
}

/* ----------------------- Casting helpers ----------------------- */

/// Trait for known types/structs which can be casted into [`Scalar`].
// TODO: handle types by value instead of reference, so we can cleanly deal with long strings.
pub trait ToScalar {
	/// Casts the type to a [`Scalar`].
	fn to_scalar(&self) -> Scalar;
}

/*
impl ToScalar for Scalar {
	fn to_scalar(&self) -> Scalar {
		// FIX ME!
		self.clone()
	}
}
*/

impl ToScalar for bool {
	fn to_scalar(&self) -> Scalar {
		Scalar::Bool(*self)
	}
}

impl ToScalar for String {
	fn to_scalar(&self) -> Scalar {
		match types::ShortString::from(self) {
			Ok(ss) => Scalar::ShortString(ss),
			Err(_) => Scalar::String(self.clone()),
		}
	}
}

impl ToScalar for &str {
	fn to_scalar(&self) -> Scalar {
		match types::ShortString::from(*self) {
			Ok(ss) => Scalar::ShortString(ss),
			Err(_) => Scalar::String((*self).into()),
		}
	}
}

impl ToScalar for Duration {
	fn to_scalar(&self) -> Scalar {
		Scalar::Uint((*self).as_secs())
	}
}

impl ToScalar for &Duration {
	fn to_scalar(&self) -> Scalar {
		Scalar::Uint((*self).as_secs())
	}
}

impl ToScalar for Timestamp {
	fn to_scalar(&self) -> Scalar {
		Scalar::Uint((*self).as_secs())
	}
}

impl ToScalar for &Timestamp {
	fn to_scalar(&self) -> Scalar {
		Scalar::Uint((*self).as_secs())
	}
}

// TODO: Switch to ShortString
impl ToScalar for thread::ThreadId {
	fn to_scalar(&self) -> Scalar {
		Scalar::String(format!("{:?}", *self))
	}
}

// TODO: Switch to ShortString
impl ToScalar for &thread::ThreadId {
	fn to_scalar(&self) -> Scalar {
		Scalar::String(format!("{:?}", *self))
	}
}

macro_rules! cast_signed_to_scalar {
	($t: ty) => {
		impl ToScalar for $t {
			fn to_scalar(&self) -> Scalar {
				Scalar::Int(*self as i64)
			}
		}
	};
}

cast_signed_to_scalar!(i8);
cast_signed_to_scalar!(i16);
cast_signed_to_scalar!(i32);
cast_signed_to_scalar!(i64);

impl ToScalar for i128 {
	fn to_scalar(&self) -> Scalar {
		Scalar::LongInt(*self)
	}
}

impl ToScalar for isize {
	fn to_scalar(&self) -> Scalar {
		Scalar::Size(*self)
	}
}

macro_rules! cast_unsigned_to_scalar {
	($t: ty) => {
		impl ToScalar for $t {
			fn to_scalar(&self) -> Scalar {
				Scalar::Uint(*self as u64)
			}
		}
	};
}

cast_unsigned_to_scalar!(u8);
cast_unsigned_to_scalar!(u16);
cast_unsigned_to_scalar!(u32);
cast_unsigned_to_scalar!(u64);

impl ToScalar for u128 {
	fn to_scalar(&self) -> Scalar {
		Scalar::LongUint(*self)
	}
}

impl ToScalar for usize {
	fn to_scalar(&self) -> Scalar {
		Scalar::Usize(*self)
	}
}

macro_rules! cast_float_to_scalar {
	($t: ty) => {
		impl ToScalar for $t {
			fn to_scalar(&self) -> Scalar {
				Scalar::Float(*self as f64)
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

impl<'i, T: ToScalar> ToScalarArray<'i, 1> for T {
	fn to_scalar_array(self) -> [Scalar; 1] {
		[Scalar::from(self)]
	}
}

impl<'i, T: ToScalar, const N: usize> ToScalarArray<'i, N> for [T; N] {
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
	fn to_scalar() {
		let short_string = "lalala";
		let long_string = "this is a rather long string, which may be complicated";

		assert_eq!(true.to_scalar(), Scalar::Bool(true));
		assert_eq!(
			short_string.to_scalar(),
			Scalar::ShortString(types::ShortString::from(short_string).expect("ShortString serialization failed"))
		);
		assert_eq!(
			String::from(short_string).to_scalar(),
			Scalar::ShortString(types::ShortString::from(short_string).expect("ShortString serialization failed"))
		);
		assert_eq!(long_string.to_scalar(), Scalar::String(long_string.into()));
		assert_eq!(String::from(long_string).to_scalar(), Scalar::String(long_string.into()));
		assert_eq!((-12 as i8).to_scalar(), Scalar::Int(-12));
		assert_eq!((345 as i16).to_scalar(), Scalar::Int(345));
		assert_eq!((-678 as i32).to_scalar(), Scalar::Int(-678));
		assert_eq!((9012 as i64).to_scalar(), Scalar::Int(9012));
		assert_eq!((-3456 as i128).to_scalar(), Scalar::LongInt(-3456));
		assert_eq!((7890 as isize).to_scalar(), Scalar::Size(7890));
		assert_eq!((12 as u8).to_scalar(), Scalar::Uint(12));
		assert_eq!((345 as u16).to_scalar(), Scalar::Uint(345));
		assert_eq!((678 as u32).to_scalar(), Scalar::Uint(678));
		assert_eq!((9012 as u64).to_scalar(), Scalar::Uint(9012));
		assert_eq!((3456 as u128).to_scalar(), Scalar::LongUint(3456));
		assert_eq!((7890 as usize).to_scalar(), Scalar::Usize(7890));
		assert_eq!(
			// yaay precision!
			(-123.456 as f32).to_scalar(),
			Scalar::Float(-123.45600128173828)
		);
		assert_eq!((789.012 as f64).to_scalar(), Scalar::Float(789.012));
		assert_eq!(Duration::from_millis(12345).to_scalar(), Scalar::Uint(12));
		assert_eq!((&Duration::from_millis(67890)).to_scalar(), Scalar::Uint(67));
		assert_eq!(Timestamp::from_millis(12345).to_scalar(), Scalar::Uint(12));
		assert_eq!((&Timestamp::from_millis(67890)).to_scalar(), Scalar::Uint(67));
	}

	#[test]
	fn dbg_format() {
		assert_eq!(format!("{}", Scalar::Bool(true)), "true");
		assert_eq!(format!("{}", Scalar::Bool(false)), "false");
		assert_eq!(format!("{}", Scalar::String("".into())), "\"\"");
		assert_eq!(format!("{}", Scalar::String("abcd 1234".into())), "\"abcd 1234\"");
		assert_eq!(format!("{}", Scalar::Int(-123)), "-123");
		assert_eq!(format!("{}", Scalar::Int(456)), "456");
		assert_eq!(format!("{}", Scalar::LongInt(-12345678901234567)), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Scalar::LongInt(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Scalar::Size(-12345678901234567)), "-0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Scalar::Size(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Scalar::Uint(123456)), "123456");
		assert_eq!(format!("{}", Scalar::LongUint(12345678901234567)), "0x2bdc545d6b4b87");
		assert_eq!(format!("{}", Scalar::Usize(89801234567890123)), "0x13f09bf3ecf84cb");
		assert_eq!(format!("{}", Scalar::Float(-1.2345)), "-1.2345");
		assert_eq!(format!("{}", Scalar::Float(6.78901)), "6.78901");
	}

	#[test]
	fn into_string() {
		let mut out = String::from("lalalala!");
		Scalar::LongInt(89801234567890123).into_string(&mut out);
		assert_eq!(out, "0x13f09bf3ecf84cb");
	}
}
