use std::fmt;
use std::io;

use crate::time::Timestamp;

pub enum StringFormat {
	UtcDateTime,       // "2026-03-02 13:22:15"
	UtcMillisDateTime, // "2026-03-02 13:22:15.488"
	UtcNanosDateTime,  // "2026-03-02 13:22:15.488728341"
	UtcTime,           // "13:22:15"
	UtcMillisTime,     // "13:22:15.488"
	UtcNanosTime,      // "13:22:15.488167982"
	UtcFileName,       // "2026-03-02_13-22-15"
	UtcRFC2822,        // "Mon, 02 Mar 2026 13:22:15 +0000"
	UtcRFC3339,        // "2026-03-02T13:22:15Z"
	UtcHTTP,           // an alias for...
	UtcRFC7231,        // "Mon, 02 Mar 2026 13:22:15 UTC"

	LocalDateTime,       // "2026-03-02 13:22:15 +0100"
	LocalMillisDateTime, // "2026-03-02 13:22:15.488 +0100"
	LocalNanosDateTime,  // "2026-03-02 13:22:15.488728341 +0100"
	LocalTime,           // "13:22:15"
	LocalMillisTime,     // "13:22:15.488"
	LocalNanosTime,      // "13:22:15.488167982"
	LocalFileName,       // "2026-03-02_13-22-15"
	LocalRFC2822,        // "Mon, 02 Mar 2026 15:22:15 +0200"
	LocalRFC3339,        // "2026-03-02T12:22:15-0100"
	LocalHTTP,           // an alias for...
	LocalRFC7231,        // "Mon, 02 Mar 2026 14:22:15 CET"

	TimestampSeconds,      // 1772795501
	TimestampMilliseconds, // 1772795501890
	TimestampNanoseconds,  // 1772795501890546
}

impl StringFormat {
	pub fn is_numeric(&self) -> bool {
		match &self {
			Self::TimestampSeconds => true,
			Self::TimestampMilliseconds => true,
			Self::TimestampNanoseconds => true,
			_ => false,
		}
	}

	pub fn is_utc(&self) -> bool {
		match &self {
			Self::UtcDateTime => true,
			Self::UtcMillisDateTime => true,
			Self::UtcNanosDateTime => true,
			Self::UtcTime => true,
			Self::UtcMillisTime => true,
			Self::UtcNanosTime => true,
			Self::UtcFileName => true,
			Self::UtcRFC2822 => true,
			Self::UtcRFC3339 => true,
			Self::UtcHTTP => true,
			Self::UtcRFC7231 => true,
			Self::TimestampSeconds => true,
			Self::TimestampMilliseconds => true,
			_ => false,
		}
	}

	pub fn write<T: io::Write>(&self, out: &mut T, ts: &Timestamp) -> io::Result<()> {
		//Result<(), io::Error> {
		let parts = if self.is_utc() { ts.as_utc_parts() } else { ts.as_local_parts() };

		match self {
			StringFormat::UtcDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
			),
			StringFormat::LocalDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02} {offset_sign}{offset_hours:02}{offset_minutes:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				offset_sign = parts.gmt_offset_sign(),
				offset_hours = parts.gmt_offset_hours(),
				offset_minutes = parts.gmt_offset_minutes(),
			),
			StringFormat::UtcMillisDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02}.{msecs:03}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
			),
			StringFormat::LocalMillisDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02}.{msecs:03} {offset_sign}{offset_hours:02}{offset_minutes:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
				offset_sign = parts.gmt_offset_sign(),
				offset_hours = parts.gmt_offset_hours(),
				offset_minutes = parts.gmt_offset_minutes(),
			),
			StringFormat::UtcNanosDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02}.{msecs:03}{nsecs:06}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
				nsecs = parts.nanoseconds,
			),
			StringFormat::LocalNanosDateTime => write!(
				out,
				"{year}-{month:02}-{day:02} {hour:02}:{mins:02}:{secs:02}.{msecs:03}{nsecs:06} {offset_sign}{offset_hours:02}{offset_minutes:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
				nsecs = parts.nanoseconds,
				offset_sign = parts.gmt_offset_sign(),
				offset_hours = parts.gmt_offset_hours(),
				offset_minutes = parts.gmt_offset_minutes(),
			),
			StringFormat::UtcFileName | StringFormat::LocalFileName => write!(
				out,
				"{year}-{month:02}-{day:02}_{hour:02}-{mins:02}-{secs:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
			),
			StringFormat::UtcTime | StringFormat::LocalTime => write!(out, "{hour:02}:{mins:02}:{secs:02}", hour = parts.hour, mins = parts.minutes, secs = parts.seconds),
			StringFormat::UtcMillisTime | StringFormat::LocalMillisTime => write!(
				out,
				"{hour:02}:{mins:02}:{secs:02}.{msecs:03}",
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
			),
			StringFormat::UtcNanosTime | StringFormat::LocalNanosTime => write!(
				out,
				"{hour:02}:{mins:02}:{secs:02}.{msecs:03}{nsecs:06}",
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				msecs = parts.milliseconds,
				nsecs = parts.nanoseconds,
			),
			StringFormat::UtcRFC2822 | StringFormat::LocalRFC2822 => write!(
				out,
				"{day_name}, {day:02} {month_name} {year} {hour:02}:{mins:02}:{secs:02} {offset_sign}{offset_hours:02}{offset_minutes:02}",
				day_name = parts.day_name(),
				day = parts.month_day,
				month_name = parts.month_name(),
				year = parts.year,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				offset_sign = parts.gmt_offset_sign(),
				offset_hours = parts.gmt_offset_hours(),
				offset_minutes = parts.gmt_offset_minutes(),
			),
			StringFormat::UtcRFC3339 => write!(
				out,
				"{year}-{month:02}-{day:02}T{hour:02}:{mins:02}:{secs:02}Z",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
			),
			StringFormat::LocalRFC3339 => write!(
				out,
				"{year}-{month:02}-{day:02}T{hour:02}:{mins:02}:{secs:02}{offset_sign}{offset_hours:02}{offset_minutes:02}",
				year = parts.year,
				month = parts.month,
				day = parts.month_day,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				offset_sign = parts.gmt_offset_sign(),
				offset_hours = parts.gmt_offset_hours(),
				offset_minutes = parts.gmt_offset_minutes(),
			),
			StringFormat::UtcHTTP | StringFormat::UtcRFC7231 | StringFormat::LocalHTTP | StringFormat::LocalRFC7231 => write!(
				out,
				"{day_name}, {day:02} {month_name} {year} {hour:02}:{mins:02}:{secs:02} {timezone}",
				day_name = parts.day_name(),
				day = parts.month_day,
				month_name = parts.month_name(),
				year = parts.year,
				hour = parts.hour,
				mins = parts.minutes,
				secs = parts.seconds,
				timezone = parts.timezone,
			),
			StringFormat::TimestampSeconds => write!(out, "{}", ts.as_secs()),
			StringFormat::TimestampMilliseconds => write!(out, "{}", ts.as_millis()),
			StringFormat::TimestampNanoseconds => write!(out, "{}", ts.as_nanos()),
		}
	}

	pub fn as_string(&self, ts: &Timestamp) -> String {
		let mut out = io::Cursor::new(Vec::new());
		if let Err(e) = self.write(&mut out, ts) {
			panic!("failed to serialize Timestamp: {}", e);
		}

		match String::from_utf8(out.into_inner()) {
			Ok(s) => s,
			Err(e) => panic!("failed to convert Timestamp to String: {}", e),
		}
	}
}

/* ----------------------- Tests ----------------------- */

#[test]
fn timestamp_as_utc_string() {
	let ts = Timestamp::from_utc_date(2026, 03, 06, 14, 43, 49, 038, 23456);

	assert_eq!(StringFormat::UtcDateTime.as_string(&ts), "2026-03-06 14:43:49");
	assert_eq!(StringFormat::UtcMillisDateTime.as_string(&ts), "2026-03-06 14:43:49.038");
	assert_eq!(StringFormat::UtcNanosDateTime.as_string(&ts), "2026-03-06 14:43:49.038023456");
	assert_eq!(StringFormat::UtcFileName.as_string(&ts), "2026-03-06_14-43-49");
	assert_eq!(StringFormat::UtcTime.as_string(&ts), "14:43:49");
	assert_eq!(StringFormat::UtcMillisTime.as_string(&ts), "14:43:49.038");
	assert_eq!(StringFormat::UtcNanosTime.as_string(&ts), "14:43:49.038023456");
	assert_eq!(StringFormat::UtcRFC2822.as_string(&ts), "Fri, 06 Mar 2026 14:43:49 +0000");
	assert_eq!(StringFormat::UtcRFC3339.as_string(&ts), "2026-03-06T14:43:49Z");
	assert_eq!(StringFormat::UtcHTTP.as_string(&ts), "Fri, 06 Mar 2026 14:43:49 UTC");
	assert_eq!(StringFormat::UtcRFC7231.as_string(&ts), "Fri, 06 Mar 2026 14:43:49 UTC");

	assert_eq!(StringFormat::TimestampSeconds.as_string(&ts), "1772808229");
	assert_eq!(StringFormat::TimestampMilliseconds.as_string(&ts), "1772808229038");
	assert_eq!(StringFormat::TimestampNanoseconds.as_string(&ts), "1772808229038023456");
}

// TODO: this test does nothing but verifying the library doesn't crash. improve :)
#[test]
fn timestamp_as_local_string() {
	let now = Timestamp::now();

	println!("RFC2822 utc:   {}", now.as_string(&StringFormat::UtcRFC2822));
	println!("RFC2822 local: {}", now.as_string(&StringFormat::LocalRFC2822));

	println!("RFC7231 utc:   {}", now.as_string(&StringFormat::UtcRFC7231));
	println!("RFC7231 local: {}", now.as_string(&StringFormat::LocalRFC7231));

	println!("RFC3339 utc:   {}", now.as_string(&StringFormat::UtcRFC3339));
	println!("RFC3339 local: {}", now.as_string(&StringFormat::LocalRFC3339));
}
