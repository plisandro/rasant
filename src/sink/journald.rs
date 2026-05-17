//! [Journald](https://www.freedesktop.org/software/systemd/man/latest/systemd-journald.service.html) logging [sink] module.
//!
//! Log updates are serialized to [journal entries](https://systemd.io/JOURNAL_NATIVE_PROTOCOL/),
//! and sent to [systemd daemons](https://systemd.io/) via UNIX local sockets, with log attribute
//! values converted to systemd journal structured fields:
//!
//!   - [Scalar][Value::Scalar]s are converted directly to field values.
//!   - [Lists][Value::List]s are converted directly to repeated field values.
//!   - [Map][Value::Map]s have no native representation in journal entries,
//!     so they get converted into a repeated field of `{key: value}`.
//!
//! Log attributes can optionally also be serialized as text, alongside the journal
//! meesage, via [`MessageFormat::WithAttributes`].
//!
//! Note that systemd journal entries don't normally display in `journalctl`
//! output, unless explicitly set to JSON.

use std::io;
use std::io::Write;
use std::os::unix::net::UnixDatagram;

use crate::attributes::{Map, Scalar, Value};
use crate::constant::{DEFUALT_JOURNALD_SOCKET, NETWORK_TIMEOUT, PROCESS_ID};
use crate::encoding;
use crate::sink;

/// Defines how journald messages are formatted.
#[derive(Debug, PartialEq)]
pub enum MessageFormat {
	/// Raw log message, unmodified.
	Raw,
	/// Attributes are appended as text at the end of the log message; note that
	/// attributes will still also be expanded as journald fields.
	WithAttributes,
}

/// Configuration struct for an journald [`sink`].
#[derive(Debug)]
pub struct JournaldConfig<'e> {
	/// Name for this sink.
	pub name: &'e str,
	/// journald socket path (f.ex. `/run/systemd/journal/socket`).
	pub path: &'e str,
	/// Message formatting.
	pub message_format: MessageFormat,
}

impl<'i> Default for JournaldConfig<'i> {
	fn default() -> Self {
		Self {
			name: "default journald",
			path: DEFUALT_JOURNALD_SOCKET,
			message_format: MessageFormat::Raw,
		}
	}
}

/// A general journald [`sink`].
pub struct Journald {
	name: String,
	message_format: MessageFormat,
	// datagram is optional only for testing purposes
	datagram: Option<UnixDatagram>,
	process_id: u32,
	output_buf: Vec<u8>,
}

impl Journald {
	/// Initializes a null [`Journald`] [`sink`]. Intended for testing.
	pub fn black_hole(conf: JournaldConfig<'_>) -> Self {
		Self {
			name: String::from(conf.name),
			message_format: conf.message_format,
			datagram: None,
			process_id: *PROCESS_ID,
			output_buf: Vec::new(),
		}
	}

	/// Initializes a new [`Journald`] [`sink`], from a given [`JournaldConfig`].
	pub fn new(conf: JournaldConfig<'_>) -> Self {
		let dg = UnixDatagram::unbound().expect("failed to initialize Unix datagram socket for syslog");
		dg.connect(conf.path).expect("failed to open journald socket for \"{path}\"");
		dg.set_write_timeout(Some(NETWORK_TIMEOUT)).expect("failed to set journald socket timeout");

		let mut sink = Self::black_hole(conf);
		sink.datagram = Some(dg);

		sink
	}

	// serializes a [`Scalar`] as text into the write buffer.
	fn write_buf_scalar(&mut self, attrs: &Map, s: &Scalar) -> io::Result<()> {
		let out = &mut self.output_buf;
		match s {
			Scalar::Bool(b) => write!(out, "={}", b),
			Scalar::String(s, _) => encoding::str_write(out, s.as_str(), &encoding::Mode::Utf8JournalDataValue),
			Scalar::StringSlice(s, _) => encoding::str_write(out, s, &encoding::Mode::Utf8JournalDataValue),
			Scalar::StringIndex(idx, _) => encoding::str_write(out, attrs.str_by_idx(*idx), &encoding::Mode::Utf8JournalDataValue),
			Scalar::Int(i) => write!(out, "={}", i),
			Scalar::LongInt(i) => write!(out, "={}", i),
			Scalar::Size(s) => write!(out, "={}", s),
			Scalar::Uint(i) => write!(out, "={}", i),
			Scalar::LongUint(u) => write!(out, "={}", u),
			Scalar::Usize(u) => write!(out, "={}", u),
			Scalar::Float(f) => write!(out, "={}", f),
		}
	}

	// serializes a [`Value`] as text into the write buffer.
	fn write_buf_value(&mut self, attrs: &Map, key: &str, val: &Value) -> io::Result<()> {
		match val {
			Value::Scalar(s) => {
				encoding::str_write(&mut self.output_buf, key, &encoding::Mode::Utf8Uppercase)?;
				self.write_buf_scalar(attrs, s)?;
				self.output_buf.write("\n".as_bytes())?;
			}
			// lists are represented as a repeated set of keys
			Value::List(ss) => {
				for s in *ss {
					encoding::str_write(&mut self.output_buf, key, &encoding::Mode::Utf8Uppercase)?;
					self.write_buf_scalar(attrs, s)?;
					self.output_buf.write("\n".as_bytes())?;
				}
			}
			// maps are represented as a repeated set of keys with JSON content
			Value::Map(mkeys, mvals) => {
				for i in 0..mkeys.len() {
					encoding::str_write(&mut self.output_buf, key, &encoding::Mode::Utf8Uppercase)?;
					write!(&mut self.output_buf, "={{{key}: {val}}}\n", key = &mkeys[i], val = &mvals[i])?;
				}
			}
		}

		Ok(())
	}

	// Serializes all attributes as journald fields into the write buffer.
	fn write_buf_attribute_fields(&mut self, attrs: &Map) -> io::Result<()> {
		for (key, val) in attrs.iter() {
			self.write_buf_value(attrs, key, &val)?;
		}

		Ok(())
	}
}

impl sink::Sink for Journald {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &Map) -> io::Result<()> {
		self.output_buf.clear();

		// TODO: add _HOSTNAME?
		write!(
			&mut self.output_buf,
			"_PID={pid}
_SOURCE_REALTIME_TIMESTAMP={timestamp}
PRIORITY={level}
MESSAGE\n",
			pid = self.process_id,
			timestamp = update.when.as_millis(),
			level = update.level.syslog_severity(),
		)?;

		// MESSAGEs must be RLEncoded, per https://systemd.io/JOURNAL_NATIVE_PROTOCOL/, as
		// we a) need to account for possible LFs in the message body, and b) don't know
		// the size of the serialized arguments, if these are appended to the log message.
		//
		// the message byte count is then spliced on the right position to comply with the
		// journald protocol.
		//
		let msg_start = self.output_buf.len();
		self.output_buf.write(update.msg.as_bytes())?;
		match self.message_format {
			MessageFormat::Raw => _ = self.output_buf.write(&[b'\n'])?,
			MessageFormat::WithAttributes => write!(&mut self.output_buf, " {}\n", attrs)?,
		};
		// the final LF doesn't count against the message size
		let msg_len = self.output_buf.len() - msg_start - 1;
		self.output_buf.splice(msg_start..msg_start, (msg_len as u64).to_le_bytes());

		self.write_buf_attribute_fields(attrs)?;

		match &self.datagram {
			Some(dg) => _ = dg.send(self.output_buf.as_slice())?,
			None => (),
		};

		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

/// Returns an intitalized journald log [`sink`]  with defaults.
pub fn default() -> Journald {
	Journald::new(JournaldConfig::default())
}

/// Returns an intitalized null-writing journald log [`sink`]  with defaults.
pub fn black_hole() -> Journald {
	Journald::black_hole(JournaldConfig::default())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use ntime::Timestamp;

	use crate::attributes::{Scalar, Value};
	use crate::level::Level;
	use crate::sink::{LogUpdate, Sink};

	#[test]
	fn output_format() {
		let want_raw = b"_PID=12345
_SOURCE_REALTIME_TIMESTAMP=1776016599123
PRIORITY=4
MESSAGE\n\x1a\0\0\0\0\0\0\0test Syslog message update
AN_INT=123
A_FLOAT=-456.789
SOME_STRING\n\x0d\0\0\0\0\0\0\0hi there! \xe2\x9d\xa4
A_LIST=349834934
A_LIST=true
A_MAP={\"key #1\": false}\nA_MAP={\"key #2\": \"weee \\u{1f494}\"}
"
		.as_slice();
		let want_with_attrs = b"_PID=12345
_SOURCE_REALTIME_TIMESTAMP=1776016599123
PRIORITY=4
MESSAGE\n\xa5\0\0\0\0\0\0\0test Syslog message update an_int=123 a_float=-456.789 some_string=\"hi there! \\u{2764}\" a_list=[0x14da0eb6, true] a_map={\"key #1\": false, \"key #2\": \"weee \\u{1f494}\"}
AN_INT=123
A_FLOAT=-456.789
SOME_STRING\n\x0d\0\0\0\0\0\0\0hi there! \xe2\x9d\xa4
A_LIST=349834934
A_LIST=true
A_MAP={\"key #1\": false}
A_MAP={\"key #2\": \"weee \\u{1f494}\"}
".as_slice();

		for tc in [(MessageFormat::Raw, want_raw), (MessageFormat::WithAttributes, want_with_attrs)] {
			let (message_format, want) = tc;

			let update = LogUpdate::new(
				Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
				Level::Warning,
				"test Syslog message update".into(),
			);

			let mut attrs = Map::new();
			attrs.insert("an_int", Value::from(123 as i32));
			attrs.insert("a_float", Value::from(-456.789));
			attrs.insert("some_string", Value::from("hi there! ❤"));
			attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));
			attrs.insert(
				"a_map",
				Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee 💔")])),
			);

			let mut sink = Journald::black_hole(JournaldConfig {
				message_format: message_format,
				..JournaldConfig::default()
			});
			sink.process_id = 12345;

			assert!(sink.log(&update, &attrs).is_ok());

			let got = &sink.output_buf;
			assert_eq!(got, want);
		}
	}
}
