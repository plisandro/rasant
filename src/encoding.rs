/// String encoding and escaping logic.
use std::io;

use crate::constant::{UTF_8_CHAR_MAX_SIZE, UTF8_BOM};

// worst case scenario is '\x{NN}' for non-ASCII characters.
const CHAR_ESCAPE_BUFFER_SIZE: usize = 6 * UTF_8_CHAR_MAX_SIZE;

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
	// vanilla UTF-8 string
	Utf8,
	// UTF-8 with byte-order mark
	Utf8Bom,
	// UTF-8 string with upper-cased characters (when possible)
	Utf8Uppercase,
	// UTF-8 with escaped characters
	Utf8Escaped,
	// UTF-8 with journald formatting for k/v data values
	Utf8JournalDataValue,
	// UTF-8 with RFC 5424 formatting for syslog PARAM-VALUEs.
	Utf8Rfc5424ParamValue,
}

/// Evaluates whether a [`char`] needs string escaping.
pub fn char_needs_escaping(c: char) -> bool {
	// replicates the logic detailed in https://doc.rust-lang.org/std/primitive.char.html#method.escape_default.
	// unforutnately, the std lib offers no methods to evaluate escaping fon individual chars without iterators :'(
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
pub fn str_needs_escaping(s: &str) -> bool {
	s.chars().any(|c| char_needs_escaping(c))
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

/// Evaluates whether a [`&str`] has line feeds (`\n`).
pub fn str_has_lf(s: &str) -> bool {
	s.chars().any(|c| c == '\n')
}

pub fn char_encode<'f>(buf: &'f mut [u8], c: char, mode: &'f Mode) -> &'f [u8] {
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
		Mode::Utf8Rfc5424ParamValue => match c {
			// https://www.rfc-editor.org/rfc/rfc5424?utm_source=chatgpt.com#section-6.3.3
			'"' => "\\\"",
			'\\' => "\\\\",
			']' => "\\]",
			_ => c.encode_utf8(buf),
		}
		.as_bytes(),
		// all oother encodings impact string generation rather than char encoding
		_ => c.encode_utf8(buf).as_bytes(),
	}
}

pub fn char_write<T: io::Write>(out: &mut T, c: char, mode: &Mode) -> io::Result<()> {
	let mut buf: [u8; CHAR_ESCAPE_BUFFER_SIZE] = [0; CHAR_ESCAPE_BUFFER_SIZE];

	out.write(char_encode(&mut buf, c, mode))?;
	Ok(())
}

pub fn str_write<T: io::Write>(out: &mut T, s: &str, mode: &Mode) -> io::Result<()> {
	match mode {
		Mode::Utf8 => {
			// `&str`s are UTF-8 encoded \o/
			out.write(s.as_bytes())?;
		}
		Mode::Utf8Bom => {
			// UTF-8 encoded strings with a byte order mark
			out.write(UTF8_BOM.as_slice())?;
			out.write(s.as_bytes())?;
		}
		Mode::Utf8JournalDataValue => {
			// Journald values are RLE; see https://systemd.io/JOURNAL_NATIVE_PROTOCOL for details.
			// "\n{string length as little-endian u64}{utf8}"
			out.write("\n".as_bytes())?;
			out.write((s.len() as u64).to_le_bytes().as_slice())?;
			out.write(s.as_bytes())?;
		}
		_ => {
			for c in s.chars() {
				char_write(out, c, mode)?;
			}
		}
	}

	Ok(())
}

pub fn write_quoted_str<T: io::Write>(out: &mut T, s: &str, mode: &Mode) -> io::Result<()> {
	out.write(&[b'"'])?;
	str_write(out, s, mode)?;
	out.write(&[b'"'])?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn str_escaping() {
		assert_eq!(str_needs_escaping(""), false);
		assert_eq!(str_needs_escaping("abcd 1234"), false);
		assert_eq!(str_needs_escaping("declaró\nen\tcontra"), true);
		assert_eq!(str_needs_escaping("so pretty ❤"), true);
	}

	#[test]
	fn str_newlines() {
		assert_eq!(str_has_lf(""), false);
		assert_eq!(str_has_lf("abcd 1234"), false);
		assert_eq!(str_has_lf("abcd\n1234"), true);
	}

	#[test]
	fn char_encoding() {
		for tc in [
			('A', Mode::Utf8, "A"),
			('A', Mode::Utf8Uppercase, "A"),
			('A', Mode::Utf8Escaped, "A"),
			('A', Mode::Utf8JournalDataValue, "A"),
			('A', Mode::Utf8Rfc5424ParamValue, "A"),
			('z', Mode::Utf8, "z"),
			('z', Mode::Utf8Uppercase, "Z"),
			('z', Mode::Utf8Escaped, "z"),
			('z', Mode::Utf8JournalDataValue, "z"),
			('z', Mode::Utf8Rfc5424ParamValue, "z"),
			('"', Mode::Utf8, "\""),
			('"', Mode::Utf8Uppercase, "\""),
			('"', Mode::Utf8Escaped, "\\\""),
			('"', Mode::Utf8JournalDataValue, "\""),
			('"', Mode::Utf8Rfc5424ParamValue, "\\\""),
			('\t', Mode::Utf8, "\t"),
			('\t', Mode::Utf8Uppercase, "\t"),
			('\t', Mode::Utf8Escaped, "\\t"),
			('\t', Mode::Utf8JournalDataValue, "\t"),
			('\t', Mode::Utf8Rfc5424ParamValue, "\t"),
			('❤', Mode::Utf8, "❤"),
			('❤', Mode::Utf8Uppercase, "❤"),
			('❤', Mode::Utf8Escaped, "\\u{2764}"),
			('❤', Mode::Utf8JournalDataValue, "❤"),
			('❤', Mode::Utf8Rfc5424ParamValue, "❤"),
			('\"', Mode::Utf8, "\""),
			('\"', Mode::Utf8Rfc5424ParamValue, "\\\""),
			('\\', Mode::Utf8, "\\"),
			('\\', Mode::Utf8Rfc5424ParamValue, "\\\\"),
			(']', Mode::Utf8, "]"),
			(']', Mode::Utf8Rfc5424ParamValue, "\\]"),
		] {
			let (c, mode, want): (char, Mode, &str) = tc;

			let mut buf: [u8; CHAR_ESCAPE_BUFFER_SIZE] = [0; CHAR_ESCAPE_BUFFER_SIZE];
			let got = char_encode(&mut buf, c, &mode);

			assert_eq!(got, want.as_bytes());
		}
	}

	#[test]
	fn string_encoding() {
		for tc in [
			("lalala ❤ 1234", Mode::Utf8, b"lalala \xe2\x9d\xa4 1234".as_slice()),
			("lalala ❤ 1234", Mode::Utf8Uppercase, b"LALALA \xe2\x9d\xa4 1234".as_slice()),
			("lalala ❤ 1234", Mode::Utf8Escaped, b"lalala \\u{2764} 1234".as_slice()),
			("lalala ❤ 1234", Mode::Utf8Bom, b"\xef\xbb\xbflalala \xe2\x9d\xa4 1234".as_slice()),
			("lalala\n❤ 1234", Mode::Utf8JournalDataValue, b"\n\x0f\x00\x00\x00\x00\x00\x00\x00lalala\n\xe2\x9d\xa4 1234".as_slice()),
			("lalala ❤ \\ \" [ ] 1234", Mode::Utf8Rfc5424ParamValue, b"lalala \xe2\x9d\xa4 \\\\ \\\" [ \\] 1234".as_slice()),
		] {
			let (s, mode, want): (&str, Mode, &[u8]) = tc;

			let mut out: Vec<u8> = Vec::new();
			assert!(str_write(&mut out, s, &mode).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}
}
