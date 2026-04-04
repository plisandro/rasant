//! Logging levels module for Rasant.
use crate::console::Color;

/// Log level definition.
///
/// [Logger][`crate::Logger`]s evaluates these in descending order, so f.ex. a log level of [`Level::Info`] includes
/// [`Level::Warning`] and [`Level::Panic`], but not [`Level::Debug`].
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
	Warning = 4,
	/// Error updates.
	Error = 3,
	/// Appplication-wide errors from which recovery is impoosible.
	Fatal = 5,
	/// Similar to [`Level::Fatal`], but the application panics right after logging the update.
	Panic = 6,
}

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
			Self::Trace => Color::Blue,
			Self::Debug => Color::Cyan,
			Self::Info => Color::Green,
			Self::Warning => Color::Yellow,
			Self::Error => Color::Red,
			Self::Fatal => Color::BrightRed,
			Self::Panic => Color::Magenta,
		}
	}

	/// Returns a string name for the level.
	pub fn as_str(&self) -> &str {
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
	pub fn as_short_str(&self) -> &str {
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
}

impl ToString for Level {
	fn to_string(&self) -> String {
		self.as_str().into()
	}
}
