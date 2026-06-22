use std::env;
use std::io;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::constant::ENV_VAR_COLORTERM;

pub enum Color {
	Default,
	Black,
	Red,
	Green,
	Yellow,
	Blue,
	Magenta,
	Cyan,
	White,
	BrightBlack,
	BrightRed,
	BrightGreen,
	BrightYellow,
	BrightBlue,
	BrightMagenta,
	BrightCyan,
	BrightWhite,
}

static COLORTERM_OK: LazyLock<bool> = LazyLock::new(|| env::var(ENV_VAR_COLORTERM).is_ok());
// COLORTERM_OVERRIDE and COLORTERM_OVERRIDE_VALUE allow forcing COLORTERM status, while introducing as little overhead as possible.
static COLORTERM_OVERRIDE: AtomicBool = AtomicBool::new(false);
static COLORTERM_OVERRIDE_VALUE: AtomicBool = AtomicBool::new(false);

// forces a return value for colorterm status
pub fn colorterm_force(b: bool) {
	COLORTERM_OVERRIDE_VALUE.store(b, Ordering::Relaxed);
	COLORTERM_OVERRIDE.store(true, Ordering::Relaxed);
}

// resets forced colorterm status.
pub fn colorterm_unforce() {
	COLORTERM_OVERRIDE.store(false, Ordering::Relaxed);
}

impl Color {
	fn supported(&self) -> bool {
		if COLORTERM_OVERRIDE.load(Ordering::Relaxed) {
			return COLORTERM_OVERRIDE_VALUE.load(Ordering::Relaxed);
		}

		*COLORTERM_OK
	}

	pub fn to_str(&self) -> &str {
		match *self {
			Self::Default => "default",
			Self::Black => "black",
			Self::Red => "red",
			Self::Green => "green",
			Self::Yellow => "yellow",
			Self::Blue => "blue",
			Self::Magenta => "magenta",
			Self::Cyan => "cyan",
			Self::White => "white",
			Self::BrightBlack => "bright black",
			Self::BrightRed => "bright red",
			Self::BrightGreen => "bright green",
			Self::BrightYellow => "bright yellow",
			Self::BrightBlue => "bright blue",
			Self::BrightMagenta => "bright magenta",
			Self::BrightCyan => "bright cyan",
			Self::BrightWhite => "bright white",
		}
	}

	pub fn to_escape_str(&self) -> &str {
		if !self.supported() {
			return "";
		}

		match *self {
			Self::Default => "\x1B[0m",
			Self::Black => "\x1B[30m",
			Self::Red => "\x1B[31m",
			Self::Green => "\x1B[32m",
			Self::Yellow => "\x1B[33m",
			Self::Blue => "\x1B[34m",
			Self::Magenta => "\x1B[35m",
			Self::Cyan => "\x1B[36m",
			Self::White => "\x1B[37m",
			Self::BrightBlack => "\x1B[90m",
			Self::BrightRed => "\x1B[91m",
			Self::BrightGreen => "\x1B[92m",
			Self::BrightYellow => "\x1B[93m",
			Self::BrightBlue => "\x1B[94m",
			Self::BrightMagenta => "\x1B[95m",
			Self::BrightCyan => "\x1B[96m",
			Self::BrightWhite => "\x1B[97m",
		}
	}

	pub fn to_bg_escape_str(&self) -> &str {
		if !self.supported() {
			return "";
		}

		match *self {
			Self::Default => "\x1B[0m",
			Self::Black => "\x1B[40m",
			Self::Red => "\x1B[41m",
			Self::Green => "\x1B[42m",
			Self::Yellow => "\x1B[43m",
			Self::Blue => "\x1B[44m",
			Self::Magenta => "\x1B[45m",
			Self::Cyan => "\x1B[46m",
			Self::White => "\x1B[47m",
			Self::BrightBlack => "\x1B[100m",
			Self::BrightRed => "\x1B[101m",
			Self::BrightGreen => "\x1B[102m",
			Self::BrightYellow => "\x1B[103m",
			Self::BrightBlue => "\x1B[104m",
			Self::BrightMagenta => "\x1B[105m",
			Self::BrightCyan => "\x1B[106m",
			Self::BrightWhite => "\x1B[107m",
		}
	}

	pub fn write<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		write!(out, "{}", self.to_escape_str())
	}

	pub fn write_bg<T: io::Write>(&self, out: &mut T) -> io::Result<()> {
		write!(out, "{}", self.to_escape_str())
	}

	pub fn write_reset<T: io::Write>(out: &mut T) -> io::Result<()> {
		Self::Default.write(out)?;
		Self::Default.write_bg(out)
	}
}

impl ToString for Color {
	fn to_string(&self) -> String {
		self.to_str().into()
	}
}

/// Computes the visible length of a string as a `&[u8]` buffer - that is, the length
/// of the string ignoring ANSI escape sequences. The string is considered as being
/// pure ASCII.
pub fn buffer_visible_length(buf: &[u8]) -> usize {
	let mut res: usize = 0;

	let mut i: usize = 0;
	while i < buf.len() {
		if buf[i] == 0x1b {
			while i < buf.len() - 1 && buf[i] != 'm' as u8 {
				i += 1;
			}
			i += 1
		} else {
			res += 1;
			i += 1;
		}
	}

	return res;
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod buffer_length {
	use super::*;

	#[test]
	fn non_color() {
		assert_eq!(buffer_visible_length("".as_bytes()), 0);
		assert_eq!(buffer_visible_length("lala".as_bytes()), 4);
		assert_eq!(buffer_visible_length("abcde 12345".as_bytes()), 11);
	}

	#[test]
	fn color() {
		assert_eq!(buffer_visible_length("\x1B[31m".as_bytes()), 0);
		assert_eq!(buffer_visible_length("\x1B[31m\x1B[0m".as_bytes()), 0);
		assert_eq!(buffer_visible_length("\x1B[34mblue\x1B[0m".as_bytes()), 4);
		assert_eq!(buffer_visible_length("\x1B[34mblue \x1B[31mRED".as_bytes()), 8);
	}
}
