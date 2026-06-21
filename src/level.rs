//! Logging levels module for Rasant.
use crate::console::Color;

/// Log level definition.
///
/// [Logger][`crate::Logger`]s evaluates these in descending order, so f.ex. a log level
/// of [`Level::Info`] includes [`Level::Warning`] and [`Level::Panic`],
/// but not [`Level::Debug`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Level {
	/// Used for tracing code execution path. Rasant will log some operations at
	/// this level, such as sink additions and log level changes.
	Trace = 0,
	/// Messages useful for debugging and troubleshooting.
	Debug = 1,
	/// Normal operation updates.
	Info = 2,
	/// Unusual events that might require attention, but do not otherwise impact normal operation.
	Warning = 3,
	/// Error updates.
	Error = 4,
	/// Appplication-wide errors from which recovery is impoosible.
	Fatal = 5,
	/// Similar to [`Level::Fatal`], but the application panics right after logging the update.
	Panic = 6,
}

const ALL_LEVELS: [Level; 7] = [Level::Trace, Level::Debug, Level::Info, Level::Warning, Level::Error, Level::Fatal, Level::Panic];

impl Level {
	/// Returns a numeric value for the log level.
	pub fn value(&self) -> u8 {
		*self as u8
	}

	/// Evaluates whether this level covers another - i.e. it's at the same, or higher level.
	pub fn covers(&self, other: &Level) -> bool {
		other.value() >= self.value()
	}

	/// Evaluates whether this level is covered by another.
	pub fn includes(&self, other: &Level) -> bool {
		other.value() <= self.value()
	}

	/// Returns a color associated with a given level, useful mostly for pretty printing.
	pub fn color(&self) -> Color {
		match *self {
			Self::Trace => Color::White,
			Self::Debug => Color::BrightBlue,
			Self::Info => Color::Green,
			Self::Warning => Color::Yellow,
			Self::Error => Color::Red,
			Self::Fatal => Color::BrightRed,
			Self::Panic => Color::Magenta,
		}
	}

	/// Returns a string name for the level.
	pub fn as_str(&self) -> &'static str {
		match *self {
			Self::Trace => "trace",
			Self::Debug => "debug",
			Self::Info => "info",
			Self::Warning => "warning",
			Self::Error => "error",
			Self::Fatal => "fatal",
			Self::Panic => "panic",
		}
	}

	/// Returns a short, three-letter name for the level.
	pub fn as_short_str(&self) -> &'static str {
		match *self {
			Self::Trace => "TRA",
			Self::Debug => "DBG",
			Self::Info => "INF",
			Self::Warning => "WRN",
			Self::Error => "ERR",
			Self::Fatal => "FAT",
			Self::Panic => "PNC",
		}
	}

	/// Returns a syslog severity for the level.
	pub fn syslog_severity(&self) -> u16 {
		match *self {
			Self::Trace => 7,   // debug
			Self::Debug => 7,   // debug
			Self::Info => 6,    // informational
			Self::Warning => 4, // warning
			Self::Error => 3,   // error
			Self::Fatal => 1,   // alert
			Self::Panic => 0,   // emergency
		}
	}
}

impl ToString for Level {
	fn to_string(&self) -> String {
		self.as_str().into()
	}
}

impl TryFrom<&str> for Level {
	type Error = &'static str;

	fn try_from(name: &str) -> Result<Self, <Level as TryFrom<&str>>::Error> {
		for l in ALL_LEVELS {
			if name.eq_ignore_ascii_case(l.as_str()) {
				return Ok(l);
			}
		}

		Err("invalid Level name")
	}
}

impl TryFrom<u8> for Level {
	type Error = &'static str;

	fn try_from(value: u8) -> Result<Self, <Level as TryFrom<u8>>::Error> {
		for l in ALL_LEVELS {
			if value == l.value() {
				return Ok(l);
			}
		}

		Err("invalid Level value")
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod from {
	use super::*;

	#[test]
	fn name() {
		assert_eq!(Level::try_from(""), Err("invalid Level name"));
		assert_eq!(Level::try_from("boo"), Err("invalid Level name"));
		assert_eq!(Level::try_from("iNfO"), Ok(Level::Info));
		assert_eq!(Level::try_from("warNINg"), Ok(Level::Warning));
		assert_eq!(Level::try_from("pnc"), Err("invalid Level name"));
		assert_eq!(Level::try_from("panic"), Ok(Level::Panic));
		assert_eq!(Level::try_from("tRa"), Err("invalid Level name"));
		assert_eq!(Level::try_from("TRACE"), Ok(Level::Trace));
	}

	#[test]
	fn value() {
		assert_eq!(Level::try_from(0), Ok(Level::Trace));
		assert_eq!(Level::try_from(1), Ok(Level::Debug));
		assert_eq!(Level::try_from(2), Ok(Level::Info));
		assert_eq!(Level::try_from(3), Ok(Level::Warning));
		assert_eq!(Level::try_from(4), Ok(Level::Error));
		assert_eq!(Level::try_from(5), Ok(Level::Fatal));
		assert_eq!(Level::try_from(6), Ok(Level::Panic));
		assert_eq!(Level::try_from(7), Err("invalid Level value"));
	}
}
