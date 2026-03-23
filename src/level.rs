use crate::console::Color;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Level {
	Trace = 0,
	Debug = 1,
	Info = 2,
	Warning = 4,
	Error = 3,
	Fatal = 5,
	Panic = 6,
}

impl Level {
	pub fn value(&self) -> u8 {
		*self as u8
	}

	pub fn covers(&self, other: &Level) -> bool {
		other.value() >= self.value()
	}

	pub fn includes(&self, other: &Level) -> bool {
		other.value() <= self.value()
	}

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
