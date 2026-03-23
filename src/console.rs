use std::env;
use std::io;
use std::sync::LazyLock;

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

static COLORTERM_OK: LazyLock<bool> = LazyLock::new(|| env::var("COLORTERM").is_ok());

impl Color {
	fn supported(&self) -> bool {
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
