use core::ffi::CStr;
use core::ffi::c_char;
use std::ptr;

use super::Timestamp;
use super::c_bindings;

use crate::time::constant::{TIMEZONE_UTC, U32_NANOS_IN_MILLI, U64_MILLIS_IN_SECOND, U64_NANOS_IN_MILLI, U128_NANOS_IN_SECOND};

#[derive(Debug, PartialEq)]
pub struct TimestampParts<'l> {
	pub nanoseconds: u32,
	pub milliseconds: u16,
	pub seconds: u8,
	pub minutes: u8,
	pub hour: u8,
	pub month_day: u8,
	pub month: u8,
	pub year: u16,
	pub week_day: u8,
	pub year_day: u16,
	pub gmt_offset_secs: i16,
	pub timezone: &'l str,
}

impl<'l> TimestampParts<'_> {
	pub fn utc(seconds: u64, nanos: u32) -> Self {
		let ts = seconds as c_bindings::CTime;
		let Some(tm) = c_bindings::c_time_to_utc_tm(ts) else {
			panic!("failed to parse UTC parts for timestamp={}s", seconds);
		};

		TimestampParts {
			nanoseconds: (nanos % U32_NANOS_IN_MILLI) as _,
			milliseconds: (nanos / U32_NANOS_IN_MILLI) as _,
			seconds: tm.tm_sec as _,
			minutes: tm.tm_min as _,
			hour: tm.tm_hour as _,
			month_day: tm.tm_mday as _,
			month: (1 + tm.tm_mon) as _,
			year: (1900 + tm.tm_year) as _,
			week_day: tm.tm_wday as _,
			year_day: tm.tm_yday as _,
			gmt_offset_secs: 0 as _,
			timezone: TIMEZONE_UTC,
		}
	}

	pub fn local(seconds: u64, nanos: u32) -> Self {
		let ts = seconds as c_bindings::CTime;
		let Some(tm) = c_bindings::c_time_to_local_tm(ts) else {
			panic!("failed to parse local parts for timestamp={}s", seconds);
		};

		let gmt_offset_secs: i16;
		let timezone: &str;
		#[cfg(not(target_env = "msvc"))]
		{
			gmt_offset_secs = tm.tm_gmtoff as _;
			let c_timezone = unsafe { CStr::from_ptr(tm.tm_zone).to_str() };
			match c_timezone {
				Ok(s) => timezone = s,
				Err(e) => panic!("failed to resolve TZ string from {tm:?}: {e}"),
			};
		}
		#[cfg(target_env = "msvc")]
		{
			(timezone, gmt_offset) = c_bindings::c_tz_info();
		}

		TimestampParts {
			nanoseconds: (nanos % U32_NANOS_IN_MILLI) as _,
			milliseconds: (nanos / U32_NANOS_IN_MILLI) as _,
			seconds: tm.tm_sec as _,
			minutes: tm.tm_min as _,
			hour: tm.tm_hour as _,
			month_day: tm.tm_mday as _,
			month: (1 + tm.tm_mon) as _,
			year: (1900 + tm.tm_year) as _,
			week_day: tm.tm_wday as _,
			year_day: tm.tm_yday as _,
			gmt_offset_secs: gmt_offset_secs,
			timezone: timezone,
		}
	}

	pub fn utc_from_secs(seconds: u64) -> Self {
		Self::utc(seconds, 0)
	}

	pub fn utc_from_millis(millis: u64) -> Self {
		Self::utc(millis / U64_MILLIS_IN_SECOND, ((millis % U64_MILLIS_IN_SECOND) * U64_NANOS_IN_MILLI) as u32)
	}

	pub fn utc_from_nanos(nanos: u128) -> Self {
		Self::utc((nanos / U128_NANOS_IN_SECOND) as _, (nanos % U128_NANOS_IN_SECOND) as _)
	}
}

impl<'l> TimestampParts<'_> {
	pub fn day_name(&self) -> &str {
		["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][(self.week_day % 8) as usize]
	}

	pub fn month_name(&self) -> &str {
		["---", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(self.month % 13) as usize]
	}

	pub fn utc_to_timestamp(&self) -> Timestamp {
		let null_c_char: *const c_char = ptr::null();
		let tm = &mut c_bindings::c_tm {
			tm_sec: self.seconds as _,
			tm_min: self.minutes as _,
			tm_hour: self.hour as _,
			tm_mday: self.month_day as _,
			tm_mon: (self.month - 1) as _,
			tm_year: (self.year - 1900) as _,
			// none of the following fields are used
			tm_wday: 0 as _,
			tm_yday: 0 as _,
			tm_isdst: 0,
			tm_gmtoff: 0,
			tm_zone: null_c_char as *mut c_char,
		};

		let secs = c_bindings::c_utc_tm_to_time(tm) as u64;
		let nanos = self.nanoseconds + ((self.milliseconds as u32) * U32_NANOS_IN_MILLI);
		super::Timestamp::new(secs, nanos)
	}

	pub fn gmt_offset_sign(&self) -> &str {
		if self.gmt_offset_secs >= 0 { "+" } else { "-" }
	}

	pub fn gmt_offset_hours(&self) -> u8 {
		(self.gmt_offset_secs / (60 * 60)) as _
	}

	pub fn gmt_offset_minutes(&self) -> u8 {
		((self.gmt_offset_secs % (60 * 60)) / 60) as _
	}
}

/* ----------------------- Tests ----------------------- */
