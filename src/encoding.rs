/// String encoding and escaping logic.
use std::io;

// worst case scenario is '\x{NN}' for non-ASCII characters.
const CHAR_ESCAPE_BUFFER_SIZE: usize = 6 * char::MAX_LEN_UTF8;

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
	Utf8,
	Utf8Uppercase,
	Utf8Escaped,
	Utf8Journald,
}

/// Evaluates whether a [`char`] needs string escaping.
pub fn needs_escaping_char(c: char) -> bool {
	// replicates the logic detailed in https://doc.rust-lang.org/std/primitive.char.html#method.escape_default.
	// unforutnately, the std lib has no way to evaluate escaping for individual chars without iterators :'(
	match c {
		'\t' => true,
		'\r' => true,
		'\n' => true,
		'\'' => true,
		'"' => true,
		_ => !c.is_ascii(),
	}
}

/// Evaluates whether a [`&str`] needs string escaping.
pub fn needs_escaping_str(s: &str) -> bool {
	s.chars().any(|c| needs_escaping_char(c))
	/*
	let mut escaped_iter = s.escape_default();
	for c in s.chars() {
		match escaped_iter.next() {
			None => return true, // oops
			Some(ec) => {
				if c != ec {
					return true;
				}
			}
		}
	}

	false
	*/
}

pub fn encode_char<'f>(buf: &'f mut [u8], c: char, mode: &'f Mode) -> &'f [u8] {
	match mode {
		Mode::Utf8 => c.encode_utf8(buf).as_bytes(),
		Mode::Utf8Uppercase => c.to_ascii_uppercase().encode_utf8(buf).as_bytes(),
		Mode::Utf8Escaped => {
			let mut start: usize = 0;
			let end: usize = buf.len();

			for c in c.escape_default() {
				let bytes = c.encode_utf8(&mut buf[start..end]).len();
				start += bytes;
			}

			&buf[0..start]
		}
		// journald expects individual chars as UTF-8
		Mode::Utf8Journald => c.encode_utf8(buf).as_bytes(),
	}
}

pub fn write_char<T: io::Write>(out: &mut T, c: char, mode: &Mode) -> io::Result<()> {
	let mut buf: [u8; _] = [0; CHAR_ESCAPE_BUFFER_SIZE];

	out.write(encode_char(&mut buf, c, mode))?;
	Ok(())
}

pub fn write_str<T: io::Write>(out: &mut T, s: &str, mode: &Mode) -> io::Result<()> {
	match mode {
		// see https://systemd.io/JOURNAL_NATIVE_PROTOCOL for details.
		Mode::Utf8Journald => {
			match s.chars().any(|c| c == '\n') {
				false => {
					// no newlines -> "={utf8}"
					out.write("=".as_bytes())?;
					write_str(out, s, &Mode::Utf8)?;
				}
				true => {
					// newlines -> "\n{string lenght as little-endian u64}{utf8}"
					out.write("\n".as_bytes())?;
					out.write((s.len() as u64).to_le_bytes().as_slice())?;
					write_str(out, s, &Mode::Utf8)?;
				}
			}
		}
		_ => {
			for c in s.chars() {
				write_char(out, c, mode)?;
			}
		}
	}

	Ok(())
}

pub fn write_quoted_str<T: io::Write>(out: &mut T, s: &str, mode: &Mode) -> io::Result<()> {
	out.write(&[b'"'])?;
	write_str(out, s, mode)?;
	out.write(&[b'"'])?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn str_escaping() {
		assert_eq!(needs_escaping_str(""), false);
		assert_eq!(needs_escaping_str("abcd 1234"), false);
		assert_eq!(needs_escaping_str("declaró\nen\tcontra"), true);
		assert_eq!(needs_escaping_str("so pretty ❤"), true);
	}

	#[test]
	fn char_encoding() {
		for tc in [
			('A', Mode::Utf8, "A"),
			('A', Mode::Utf8Uppercase, "A"),
			('A', Mode::Utf8Escaped, "A"),
			('A', Mode::Utf8Journald, "A"),
			('z', Mode::Utf8, "z"),
			('z', Mode::Utf8Uppercase, "Z"),
			('z', Mode::Utf8Escaped, "z"),
			('z', Mode::Utf8Journald, "z"),
			('"', Mode::Utf8, "\""),
			('"', Mode::Utf8Uppercase, "\""),
			('"', Mode::Utf8Escaped, "\\\""),
			('"', Mode::Utf8Journald, "\""),
			('\t', Mode::Utf8, "\t"),
			('\t', Mode::Utf8Uppercase, "\t"),
			('\t', Mode::Utf8Escaped, "\\t"),
			('\t', Mode::Utf8Journald, "\t"),
			('❤', Mode::Utf8, "❤"),
			('❤', Mode::Utf8Uppercase, "❤"),
			('❤', Mode::Utf8Escaped, "\\u{2764}"),
			('❤', Mode::Utf8Journald, "❤"),
		] {
			let (c, mode, want): (char, Mode, &str) = tc;

			let mut buf: [u8; _] = [0; CHAR_ESCAPE_BUFFER_SIZE];
			let got = encode_char(&mut buf, c, &mode);

			assert_eq!(got, want.as_bytes());
		}
	}

	#[test]
	fn string_encoding() {
		for tc in [
			("lalala ❤ 1234", Mode::Utf8, "lalala ❤ 1234".as_bytes()),
			("lalala ❤ 1234", Mode::Utf8Uppercase, "LALALA ❤ 1234".as_bytes()),
			("lalala ❤ 1234", Mode::Utf8Escaped, "lalala \\u{2764} 1234".as_bytes()),
			("lalala ❤ 1234", Mode::Utf8Journald, "=lalala ❤ 1234".as_bytes()),
			(
				"lalala\n1234",
				Mode::Utf8Journald,
				&[0x0a, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6c, 0x61, 0x6c, 0x61, 0x6c, 0x61, 0x0a, 0x31, 0x32, 0x33, 0x34],
			),
		] {
			let (s, mode, want): (&str, Mode, &[u8]) = tc;

			let mut out: Vec<u8> = Vec::new();
			assert!(write_str(&mut out, s, &mode).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}
}
