/// String encoding and escaping logic.
use std::io;

use crate::constant::{UTF_8_CHAR_MAX_SIZE, UTF8_BOM};

// worst case scenario is '\x{NN}' for non-ASCII characters.
const CHAR_ESCAPE_BUFFER_SIZE: usize = 6 * UTF_8_CHAR_MAX_SIZE;

#[derive(Clone, Debug, PartialEq)]
// Allow enums for journald/syslog-specific strings on non-Unix platforms, even
// when not actively used.
#[allow(dead_code)]
pub enum Mode {
	/// Vanilla UTF-8 string
	Utf8,
	/// UTF-8 with byte-order mark.
	Utf8Bom,
	/// UTF-8 string with upper-cased characters (when possible).
	Utf8Uppercase,
	/// UTF-8 with escaped characters.
	Utf8Escaped,
	/// UTF-8 with journald formatting for k/v data values.
	Utf8JournalDataValue,
	/// UTF-8 with RFC 5424 formatting for syslog PARAM-VALUEs.
	Utf8Rfc5424ParamValue,
	/// RFC 8259 - a.k.a JSON strings.
	Rfc8259,
}

/// Evaluates whether a [`char`] needs string escaping.
pub fn char_needs_escaping(c: char) -> bool {
	// replicates the logic detailed in https://doc.rust-lang.org/std/primitive.char.html#method.escape_default.
	// unfortunately, the std lib offers no methods to evaluate escaping for individual chars without iterators :'(
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
// TODO: This check is valid only for UTF8* encodings. Rework Scalars with string escaping annotations - or maybe just drop this?
pub fn str_needs_escaping(s: &str) -> bool {
	s.chars().any(char_needs_escaping)
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
		// https://www.rfc-editor.org/rfc/rfc5424?utm_source=chatgpt.com#section-6.3.3
		Mode::Utf8Rfc5424ParamValue => match c {
			'"' => b"\\\"",
			'\\' => b"\\\\",
			']' => b"\\]",
			_ => c.encode_utf8(buf).as_bytes(),
		},
		// https://www.rfc-editor.org/info/rfc8259/#section-7
		Mode::Rfc8259 => match c {
			'"' => b"\\u0022",
			'\\' => b"\\u005c",
			'/' => b"\\u002f",
			// backspace
			'\x08' => b"\\u0008",
			// form feed
			'\x0c' => b"\\u000c",
			'\n' => b"\\u000a",
			'\r' => b"\\u000d",
			'\t' => b"\\u0009",
			// other escape characters (0x00 to 0x1f)
			'\x00' => b"\\u0000",
			'\x01' => b"\\u0001",
			'\x02' => b"\\u0002",
			'\x03' => b"\\u0003",
			'\x04' => b"\\u0004",
			'\x05' => b"\\u0005",
			'\x06' => b"\\u0006",
			'\x07' => b"\\u0007",
			'\x0b' => b"\\u000b",
			'\x0e' => b"\\u000e",
			'\x0f' => b"\\u000f",
			'\x10' => b"\\u0010",
			'\x11' => b"\\u0011",
			'\x12' => b"\\u0012",
			'\x13' => b"\\u0013",
			'\x14' => b"\\u0014",
			'\x15' => b"\\u0015",
			'\x16' => b"\\u0016",
			'\x17' => b"\\u0017",
			'\x18' => b"\\u0018",
			'\x19' => b"\\u0019",
			'\x1a' => b"\\u001a",
			'\x1b' => b"\\u001b",
			'\x1c' => b"\\u001c",
			'\x1d' => b"\\u001d",
			'\x1e' => b"\\u001e",
			'\x1f' => b"\\u001f",
			// UTF-8 bytes are valid within JSON strings
			_ => c.encode_utf8(buf).as_bytes(),
		},
		// all other encodings impact string generation rather than char encoding
		_ => c.encode_utf8(buf).as_bytes(),
	}
}

pub fn char_write<T: io::Write>(out: &mut T, c: char, mode: &Mode) -> io::Result<()> {
	let mut buf: [u8; CHAR_ESCAPE_BUFFER_SIZE] = [0; CHAR_ESCAPE_BUFFER_SIZE];

	out.write_all(char_encode(&mut buf, c, mode))?;
	Ok(())
}

pub fn str_write<T: io::Write>(out: &mut T, s: &str, mode: &Mode) -> io::Result<()> {
	match mode {
		Mode::Utf8 => {
			// `&str`s are UTF-8 encoded \o/
			out.write_all(s.as_bytes())?;
		}
		Mode::Utf8Bom => {
			// UTF-8 encoded strings with a byte order mark
			out.write_all(UTF8_BOM.as_slice())?;
			out.write_all(s.as_bytes())?;
		}
		Mode::Utf8JournalDataValue => {
			// Journald values are RLE; see https://systemd.io/JOURNAL_NATIVE_PROTOCOL for details.
			// "\n{string length as little-endian u64}{utf8}"
			out.write_all(b"\n")?;
			out.write_all((s.len() as u64).to_le_bytes().as_slice())?;
			out.write_all(s.as_bytes())?;
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
	out.write_all(b"\"")?;
	str_write(out, s, mode)?;
	out.write_all(b"\"")?;

	Ok(())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn str_escaping() {
		assert!(!str_needs_escaping(""));
		assert!(!str_needs_escaping("abcd 1234"));
		assert!(str_needs_escaping("declaró\nen\tcontra"));
		assert!(str_needs_escaping("so pretty ❤"));
	}

	#[test]
	fn char_encoding() {
		for tc in [
			('A', Mode::Utf8, "A"),
			('A', Mode::Utf8Uppercase, "A"),
			('A', Mode::Utf8Escaped, "A"),
			('A', Mode::Utf8JournalDataValue, "A"),
			('A', Mode::Utf8Rfc5424ParamValue, "A"),
			('A', Mode::Rfc8259, "A"),
			('z', Mode::Utf8, "z"),
			('z', Mode::Utf8Uppercase, "Z"),
			('z', Mode::Utf8Escaped, "z"),
			('z', Mode::Utf8JournalDataValue, "z"),
			('z', Mode::Utf8Rfc5424ParamValue, "z"),
			('z', Mode::Rfc8259, "z"),
			('"', Mode::Utf8, "\""),
			('"', Mode::Utf8Uppercase, "\""),
			('"', Mode::Utf8Escaped, "\\\""),
			('"', Mode::Utf8JournalDataValue, "\""),
			('"', Mode::Utf8Rfc5424ParamValue, "\\\""),
			('"', Mode::Rfc8259, "\\u0022"),
			('\t', Mode::Utf8, "\t"),
			('\t', Mode::Utf8Uppercase, "\t"),
			('\t', Mode::Utf8Escaped, "\\t"),
			('\t', Mode::Utf8JournalDataValue, "\t"),
			('\t', Mode::Utf8Rfc5424ParamValue, "\t"),
			('\t', Mode::Rfc8259, "\\u0009"),
			('❤', Mode::Utf8, "❤"),
			('❤', Mode::Utf8Uppercase, "❤"),
			('❤', Mode::Utf8Escaped, "\\u{2764}"),
			('❤', Mode::Utf8JournalDataValue, "❤"),
			('❤', Mode::Utf8Rfc5424ParamValue, "❤"),
			('❤', Mode::Rfc8259, "❤"),
			('\"', Mode::Utf8, "\""),
			('\"', Mode::Utf8Rfc5424ParamValue, "\\\""),
			('\"', Mode::Rfc8259, "\\u0022"),
			('\\', Mode::Utf8, "\\"),
			('\\', Mode::Utf8Rfc5424ParamValue, "\\\\"),
			('\\', Mode::Rfc8259, "\\u005c"),
			(']', Mode::Utf8, "]"),
			(']', Mode::Utf8Rfc5424ParamValue, "\\]"),
			(']', Mode::Rfc8259, "]"),
		] {
			let (c, mode, want): (char, Mode, &str) = tc;

			let mut buf: [u8; CHAR_ESCAPE_BUFFER_SIZE] = [0; CHAR_ESCAPE_BUFFER_SIZE];
			let got = char_encode(&mut buf, c, &mode);

			assert_eq!(got, want.as_bytes(), "expected {} for {}", want, c);
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
			("lalala ❤ \\ \" [ ] 1234", Mode::Rfc8259, b"lalala \xe2\x9d\xa4 \\u005c \\u0022 [ ] 1234".as_slice()),
		] {
			let (s, mode, want): (&str, Mode, &[u8]) = tc;

			let mut out: Vec<u8> = Vec::new();
			assert!(str_write(&mut out, s, &mode).is_ok());
			assert_eq!(out.as_slice(), want);
		}
	}
}
