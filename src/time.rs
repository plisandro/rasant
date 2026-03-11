mod c_bindings;
mod constant;
mod format;
mod parts;

use core::ops::Sub;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;
use std::time;

use constant::{TIMEZONE_UTC, U128_MILLIS_IN_SECOND, U128_NANOS_IN_MILLI, U128_NANOS_IN_SECOND};
use parts::TimestampParts;

pub use format::StringFormat;
pub use time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp {
	seconds: u64,
	nanoseconds: u32,
}

impl Timestamp {
	pub fn new(secs: u64, nanos: u32) -> Self {
		Self { seconds: secs, nanoseconds: nanos }
	}

	pub fn from_secs(secs: u64) -> Self {
		Self { seconds: secs, nanoseconds: 0 }
	}

	pub fn from_millis(msecs: u128) -> Self {
		Self {
			seconds: (msecs / U128_MILLIS_IN_SECOND) as _,
			nanoseconds: ((msecs % U128_MILLIS_IN_SECOND) * U128_NANOS_IN_MILLI) as _,
		}
	}

	pub fn from_nanos(nanos: u128) -> Self {
		Self {
			seconds: (nanos / U128_NANOS_IN_SECOND) as _,
			nanoseconds: (nanos % U128_NANOS_IN_SECOND) as _,
		}
	}

	pub fn from_utc_date(year: u16, month: u8, day: u8, hour: u8, minutes: u8, secs: u8, millis: u16, nanos: u32) -> Self {
		TimestampParts {
			nanoseconds: nanos,
			milliseconds: millis,
			seconds: secs,
			minutes: minutes,
			hour: hour,
			month_day: day,
			month: month,
			year: year,
			week_day: 0,
			year_day: 0,
			gmt_offset_secs: 0,
			timezone: TIMEZONE_UTC,
		}
		.utc_to_timestamp()
	}

	pub fn from_system_time(time: std::time::SystemTime) -> Self {
		match time.duration_since(time::UNIX_EPOCH) {
			Ok(d) => Self::from_nanos(d.as_nanos()),
			Err(e) => panic!("failed to parse time duration: {e}"),
		}
	}

	pub fn now() -> Self {
		Self::from_system_time(time::SystemTime::now())
	}

	pub fn as_secs(&self) -> u64 {
		self.seconds
	}

	pub fn as_millis(&self) -> u128 {
		(self.seconds as u128) * U128_MILLIS_IN_SECOND + (self.nanoseconds as u128 / U128_NANOS_IN_MILLI)
	}

	pub fn as_nanos(&self) -> u128 {
		(self.seconds as u128) * U128_NANOS_IN_SECOND + self.nanoseconds as u128
	}

	pub fn as_utc_parts(&self) -> TimestampParts {
		TimestampParts::utc(self.seconds, self.nanoseconds)
	}

	pub fn as_local_parts(&self) -> TimestampParts {
		TimestampParts::local(self.seconds, self.nanoseconds)
	}

	pub fn as_string(&self, format: &StringFormat) -> String {
		format.timestamp_as_string(self)
	}

	pub fn add_duration(&mut self, d: &Duration) -> &Self {
		let nanos = d.as_nanos() + self.nanoseconds as u128;

		self.seconds = self.seconds + (nanos / U128_NANOS_IN_SECOND) as u64;
		self.nanoseconds = (nanos % U128_NANOS_IN_SECOND) as u32;

		self
	}

	fn cmp(&self, other: &Self) -> Ordering {
		if self.seconds == other.seconds {
			if self.nanoseconds < other.nanoseconds {
				return Ordering::Less;
			}
			if self.nanoseconds > other.nanoseconds {
				return Ordering::Greater;
			}
			return Ordering::Equal;
		}

		if self.seconds < other.seconds {
			return Ordering::Less;
		}
		Ordering::Greater
	}

	fn diff_as_duration(&self, other: &Self) -> Duration {
		let self_nanos = self.as_nanos();
		let other_nanos = other.as_nanos();

		if other_nanos >= self_nanos {
			Duration::ZERO
		} else {
			Duration::from_nanos((self_nanos - other_nanos) as u64)
		}
	}
}

impl fmt::Display for Timestamp {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_string(&StringFormat::LocalDateTime))
	}
}

impl PartialOrd for Timestamp {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Timestamp {
	fn cmp(&self, other: &Self) -> Ordering {
		self.cmp(other)
	}
}

impl Sub for Timestamp {
	type Output = Duration;

	fn sub(self, other: Self) -> Self::Output {
		self.diff_as_duration(&other)
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_date() {
		assert_eq!(
			Timestamp::from_utc_date(2026, 03, 07, 04, 48, 17, 446, 37892),
			Timestamp {
				seconds: 1772858897,
				nanoseconds: 446037892,
			},
			"cast from UTC date"
		);
	}

	#[test]
	fn comparison() {
		assert_eq!(Timestamp::new(1234, 100), Timestamp::new(1234, 100), "equal");
		assert!(Timestamp::new(1234, 100) > Timestamp::new(1234, 50), "same secs, more millis");
		assert!(Timestamp::new(1234, 100) < Timestamp::new(1234, 200), "same secs, less millis");
		assert!(Timestamp::new(5678, 100) > Timestamp::new(1234, 100), "more secs");
		assert!(Timestamp::new(1234, 100) < Timestamp::new(5768, 100), "less secs");
	}

	#[test]
	fn operators() {
		assert_eq!(Timestamp::from_nanos(1234) - Timestamp::from_nanos(1234), Duration::ZERO, "zero result");
		assert_eq!(Timestamp::from_nanos(1234) - Timestamp::from_nanos(5768), Duration::ZERO, "underflow");
		assert_eq!(Timestamp::from_nanos(5678) - Timestamp::from_nanos(1234), Duration::from_nanos(4444), "OK");
	}

	#[test]
	fn casting() {
		let ts = Timestamp::new(1772457319, 38123456);
		assert_eq!(ts.as_secs(), 1772457319, "cast to seconds");
		assert_eq!(ts.as_millis(), 1772457319038, "cast to millis");
		assert_eq!(ts.as_nanos(), 1772457319038123456, "cast to nanos");
	}

	// TODO: fix me
	/*
	#[test]
	fn to_string() {
		assert_eq!(
			Timestamp::new(1772457020, 789).to_string(),
			"2026-03-02 13:10:20.789"
		);
		assert_eq!(
			Timestamp::from_secs(1772457213).to_string(),
			"2026-03-02 13:13:33.000",
		);
		assert_eq!(
			Timestamp::from_millis(1772457213123).to_string(),
			"2026-03-02 13:13:33.123",
		);
		assert_eq!(
			Timestamp::from_utc_date(2026, 03, 06, 14, 43, 39, 128).to_string(),
			"2026-03-06 14:43:39.128",
		);
	}
	*/

	#[test]
	fn utc_parts_conversion() {
		assert_eq!(
			Timestamp::from_millis(1772457319335).as_utc_parts(),
			TimestampParts {
				nanoseconds: 0,
				milliseconds: 335,
				seconds: 19,
				minutes: 15,
				hour: 13,
				month_day: 2,
				month: 3,
				year: 2026,
				week_day: 1,
				year_day: 60,
				gmt_offset_secs: 0,
				timezone: TIMEZONE_UTC,
			},
			"UTC parts from milliseconds timestamp"
		);

		assert_eq!(
			Timestamp::from_nanos(1772457319335012345).as_utc_parts(),
			TimestampParts {
				nanoseconds: 12345,
				milliseconds: 335,
				seconds: 19,
				minutes: 15,
				hour: 13,
				month_day: 2,
				month: 3,
				year: 2026,
				week_day: 1,
				year_day: 60,
				gmt_offset_secs: 0,
				timezone: TIMEZONE_UTC,
			},
			"UTC parts from nanoseconds timestamp"
		);

		// TODO: move me somewhere else.
		assert_eq!(
			TimestampParts {
				nanoseconds: 123456,
				milliseconds: 320,
				seconds: 15,
				minutes: 22,
				hour: 5,
				month_day: 8,
				month: 3,
				year: 2026,
				week_day: 0,
				year_day: 0,
				gmt_offset_secs: 0,
				timezone: TIMEZONE_UTC,
			}
			.utc_to_timestamp(),
			Timestamp::from_nanos(1772947335320123456),
		);
	}

	#[test]
	fn add_duration() {
		let mut a = Timestamp::new(1234, 5678);
		a.add_duration(&Duration::from_millis(2234));
		assert_eq!(a, Timestamp::new(1236, 234005678));
	}
}
