use ntime::{Duration, Timestamp};
use std::fmt;
use std::net;
use std::string;
use std::thread;

use crate::attributes::Map;
use crate::encoding;
use crate::level::Level;

/// [Scalar] definitions for all log operations.
/// Scalars are the basic data units for Rasant, representing a single type.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
	/// A [`bool`]ean.
	Bool(bool),
	/// An owned [`String`], and whether it has characters which need escaping.
	/// This type is used for ingestion only. Attribute maps will never yield it.
	String(string::String, bool),
	/// A static [`&`str`]ing, and whether it has characters which need escaping.
	StringSlice(&'static str, bool),
	/// An indexed string, stored in an attribute map, and whether it has characters which need escaping.
	StringIndex(usize, bool),
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
	/// A pointer-sized unsigned integer, stored as a [`usize`].
	Usize(usize),
	/// A float, internally stored as a [`f64`].
	Float(f64),
}

/* ----------------------- Implementation ----------------------- */

impl fmt::Display for Scalar {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self {
			Self::Bool(b) => write!(f, "{}", b),
			Scalar::String(s, escape) => match *escape {
				false => write!(f, "\"{}\"", s),
				true => write!(f, "\"{}\"", s.as_str().escape_default()),
			},
			Scalar::StringSlice(s, escape) => match *escape {
				false => write!(f, "\"{}\"", s),
				true => write!(f, "\"{}\"", s.escape_default()),
			},
			Scalar::StringIndex(idx, escape) => write!(f, "<indexed str #{idx}, {}>", if *escape { "escaped" } else { "non-escaped" }),
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
	/// Writes a raw string representation of a [`Scalar`] into an [`fmt::Write`].
	pub fn write_fmt_raw<T: fmt::Write>(&self, out: &mut T, attrs: &Map) -> fmt::Result {
		match self {
			Self::Bool(b) => write!(out, "{}", b),
			Scalar::String(s, _) => write!(out, "{}", s),
			Scalar::StringSlice(s, _) => write!(out, "{}", s),
			Scalar::StringIndex(idx, _) => write!(out, "{}", attrs.str_by_idx(*idx)),
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

	/// Writes a string representation of a [`Scalar`] into an [`fmt::Write`].
	pub fn write_fmt<T: fmt::Write>(&self, out: &mut T, attrs: &Map) -> fmt::Result {
		match self {
			Scalar::String(s, escape) => match *escape {
				false => write!(out, "\"{}\"", s),
				true => write!(out, "\"{}\"", s.as_str().escape_default()),
			},
			Scalar::StringSlice(s, escape) => match *escape {
				false => write!(out, "\"{}\"", s),
				true => write!(out, "\"{}\"", s.escape_default()),
			},
			Scalar::StringIndex(idx, escape) => match *escape {
				false => write!(out, "\"{}\"", attrs.str_by_idx(*idx)),
				true => write!(out, "\"{}\"", attrs.str_by_idx(*idx).escape_default()),
			},
			_ => self.write_fmt_raw(out, attrs),
		}
	}

	/// Serializes a [`Scalar`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_string(&self, out: &mut String, attrs: &Map) {
		out.clear();
		self.write_fmt(out, attrs).expect("failed to serialize Scalar into_string()");
	}

	/// Serializes a raw [`Scalar`] into a pre-existing [`String`], whose contents are overwritten.
	pub fn into_raw_string(&self, out: &mut String, attrs: &Map) {
		out.clear();
		self.write_fmt_raw(out, attrs).expect("failed to serialize Scalar into_raw_string()");
	}

	/// Creates an array of [`Scalar`]s from a suitable type.
	pub fn to_array<const N: usize, T: ToScalarArray<'i, N>>(v: T) -> [Self; N] {
		v.to_scalar_array()
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
		let escaped = encoding::str_needs_escaping(s.as_str());
		Self::String(s, escaped)
	}
}

impl From<&'static str> for Scalar {
	fn from(s: &'static str) -> Self {
		Self::StringSlice(s, encoding::str_needs_escaping(s))
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
		Scalar::from(format!("{:?}", t))
	}
}

impl From<&thread::ThreadId> for Scalar {
	fn from(t: &thread::ThreadId) -> Self {
		Scalar::from(format!("{:?}", t))
	}
}

impl From<Level> for Scalar {
	fn from(l: Level) -> Self {
		Scalar::from(l.as_str())
	}
}

impl From<&Level> for Scalar {
	fn from(l: &Level) -> Self {
		Scalar::from(l.as_str())
	}
}

impl From<&net::Ipv4Addr> for Scalar {
	fn from(s: &net::Ipv4Addr) -> Self {
		Scalar::from(s.to_string())
	}
}

impl From<&net::Ipv6Addr> for Scalar {
	fn from(s: &net::Ipv6Addr) -> Self {
		Scalar::from(s.to_string())
	}
}

impl From<&net::IpAddr> for Scalar {
	fn from(s: &net::IpAddr) -> Self {
		Scalar::from(s.to_string())
	}
}

impl From<&net::SocketAddrV4> for Scalar {
	fn from(s: &net::SocketAddrV4) -> Self {
		Scalar::from(s.to_string())
	}
}

impl From<&net::SocketAddrV6> for Scalar {
	fn from(s: &net::SocketAddrV6) -> Self {
		Scalar::from(s.to_string())
	}
}

impl From<&net::SocketAddr> for Scalar {
	fn from(s: &net::SocketAddr) -> Self {
		Scalar::from(s.to_string())
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
	fn from_string() {
		assert_eq!(Scalar::from(true), Scalar::Bool(true));
		assert_eq!(Scalar::from("lalala"), Scalar::StringSlice("lalala", false));
		assert_eq!(Scalar::from("declaró\nen\tcontra"), Scalar::StringSlice("declaró\nen\tcontra", true));
		assert_eq!(Scalar::from(String::from("lalala")), Scalar::String(String::from("lalala"), false));
		assert_eq!(Scalar::from(String::from("declaró\nen\tcontra")), Scalar::String(String::from("declaró\nen\tcontra"), true));
	}

	#[test]
	fn from_base_type() {
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
	}

	#[test]
	fn from_ntime() {
		assert_eq!(Scalar::from(Duration::from_millis(12345)), Scalar::Uint(12));
		assert_eq!(Scalar::from(&Duration::from_millis(67890)), Scalar::Uint(67));
		assert_eq!(Scalar::from(Timestamp::from_millis(12345)), Scalar::Uint(12));
		assert_eq!(Scalar::from(&Timestamp::from_millis(67890)), Scalar::Uint(67));
	}

	#[test]
	fn from_net() {
		let ip4 = net::Ipv4Addr::new(12, 34, 56, 78);
		let ip6 = net::Ipv6Addr::new(0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D);

		assert_eq!(Scalar::from(&ip4), Scalar::String(String::from("12.34.56.78"), false));
		assert_eq!(Scalar::from(&ip6), Scalar::String(String::from("1020:3040:5060:7080:90a0:b0c0:d0e0:f00d"), false));
		assert_eq!(Scalar::from(&net::IpAddr::V4(ip4)), Scalar::String(String::from("12.34.56.78"), false));
		assert_eq!(Scalar::from(&net::IpAddr::V6(ip6)), Scalar::String(String::from("1020:3040:5060:7080:90a0:b0c0:d0e0:f00d"), false));

		let addr4 = net::SocketAddrV4::new(net::Ipv4Addr::new(12, 34, 56, 78), 7777);
		let addr6 = net::SocketAddrV6::new(net::Ipv6Addr::new(0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D), 8888, 1, 2);

		assert_eq!(Scalar::from(&addr4), Scalar::String(String::from("12.34.56.78:7777"), false));
		assert_eq!(Scalar::from(&addr6), Scalar::String(String::from("[1020:3040:5060:7080:90a0:b0c0:d0e0:f00d%2]:8888"), false));
		assert_eq!(Scalar::from(&net::SocketAddr::V4(addr4)), Scalar::String(String::from("12.34.56.78:7777"), false));
		assert_eq!(
			Scalar::from(&net::SocketAddr::V6(addr6)),
			Scalar::String(String::from("[1020:3040:5060:7080:90a0:b0c0:d0e0:f00d%2]:8888"), false)
		);
	}

	#[test]
	fn to_string() {
		for tc in [
			(Scalar::Bool(true), "true"),
			(Scalar::Bool(false), "false"),
			(Scalar::String(String::from(""), false), "\"\""),
			(Scalar::String(String::from("abcd 1234"), false), "\"abcd 1234\""),
			(Scalar::String(String::from("abcd 1234"), true), "\"abcd 1234\""),
			(Scalar::String(String::from("declaró\nen\tcontra"), false), "\"declaró\nen\tcontra\""),
			(Scalar::String(String::from("declaró\nen\tcontra"), true), "\"declar\\u{f3}\\nen\\tcontra\""),
			(Scalar::StringSlice("", false), "\"\""),
			(Scalar::StringSlice("abcd 1234", false), "\"abcd 1234\""),
			(Scalar::StringSlice("abcd 1234", true), "\"abcd 1234\""),
			(Scalar::StringSlice("declaró\nen\tcontra", false), "\"declaró\nen\tcontra\""),
			(Scalar::StringSlice("declaró\nen\tcontra", true), "\"declar\\u{f3}\\nen\\tcontra\""),
			(Scalar::StringIndex(123, false), "<indexed str #123, non-escaped>"),
			(Scalar::StringIndex(456, true), "<indexed str #456, escaped>"),
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

			assert_eq!(s.to_string(), want);
		}
	}

	#[test]
	fn into_string() {
		for tc in [
			(Scalar::Bool(true), "true", "true"),
			(Scalar::Bool(false), "false", "false"),
			(Scalar::String(String::from(""), false), "\"\"", ""),
			(Scalar::String(String::from("abcd 1234"), false), "\"abcd 1234\"", "abcd 1234"),
			(Scalar::String(String::from("abcd 1234"), true), "\"abcd 1234\"", "abcd 1234"),
			(Scalar::String(String::from("declaró\nen\tcontra"), false), "\"declaró\nen\tcontra\"", "declaró\nen\tcontra"),
			(Scalar::String(String::from("declaró\nen\tcontra"), true), "\"declar\\u{f3}\\nen\\tcontra\"", "declaró\nen\tcontra"),
			(Scalar::StringSlice("", false), "\"\"", ""),
			(Scalar::StringSlice("abcd 1234", false), "\"abcd 1234\"", "abcd 1234"),
			(Scalar::StringSlice("abcd 1234", true), "\"abcd 1234\"", "abcd 1234"),
			(Scalar::StringSlice("declaró\nen\tcontra", false), "\"declaró\nen\tcontra\"", "declaró\nen\tcontra"),
			(Scalar::StringSlice("declaró\nen\tcontra", true), "\"declar\\u{f3}\\nen\\tcontra\"", "declaró\nen\tcontra"),
			(Scalar::Int(-123), "-123", "-123"),
			(Scalar::Int(456), "456", "456"),
			(Scalar::LongInt(-12345678901234567), "-0x2bdc545d6b4b87", "-0x2bdc545d6b4b87"),
			(Scalar::LongInt(89801234567890123), "0x13f09bf3ecf84cb", "0x13f09bf3ecf84cb"),
			(Scalar::Size(-12345678901234567), "-0x2bdc545d6b4b87", "-0x2bdc545d6b4b87"),
			(Scalar::Size(89801234567890123), "0x13f09bf3ecf84cb", "0x13f09bf3ecf84cb"),
			(Scalar::Uint(123456), "123456", "123456"),
			(Scalar::LongUint(12345678901234567), "0x2bdc545d6b4b87", "0x2bdc545d6b4b87"),
			(Scalar::Usize(89801234567890123), "0x13f09bf3ecf84cb", "0x13f09bf3ecf84cb"),
			(Scalar::Float(-1.2345), "-1.2345", "-1.2345"),
			(Scalar::Float(6.78901), "6.78901", "6.78901"),
		] {
			let (s, want, want_raw): (Scalar, &str, &str) = tc;

			let mut out = String::from("lalalala!");
			let mut out_raw = String::from("lalalala!");
			let attrs = Map::new();

			s.into_string(&mut out, &attrs);
			s.into_raw_string(&mut out_raw, &attrs);
			assert_eq!(out, want);
			assert_eq!(out_raw, want_raw);
		}
	}
}
