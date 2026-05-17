//! Syslog logging [`sink`] module.
//!
//! Provides syslog [RFC 5424](<https://datatracker.ietf.org/doc/html/rfc5424>) and
//! [RFC 3164](<https://datatracker.ietf.org/doc/html/rfc3164>) sinks over multiple
//! socket types:
//!
//!   - Unix datagram: [`SyslogSocket::Local`] and [`SyslogSocket::LocalPath`]. Only
//!     available for *NIX builds.
//!   - UDP, implementing [RFC 5426](https://datatracker.ietf.org/doc/html/rfc5426):
//!     [`SyslogSocket::UDP`].
//!   - TCP, implementing [RFC 6587](https://datatracker.ietf.org/doc/html/rfc6587)
//!     and supporting two modes: framed ([`SyslogSocket::TCP`]) and "non-transparent
//!     framing" ([`SyslogSocket::TCPTransparent`]). Framed TCP should be favored
//!     whenever possible.
//!
//! Multiple syslog formats are supported:
//!
//!   - [`SyslogFormat::RFC3164`] implements the older, but widely supported,
//!     [RFC 3164](<https://datatracker.ietf.org/doc/html/rfc3164>) BSD standard,
//!     pretty much guaranteed to work with all existing syslog server implementations.
//!     It has a number of limitations though...
//!
//!       - Rasant log attributes are rendered as part of the syslog message.
//!       - Log timestamp precision is seconds.
//!       - Can't properly handle newlines (ASCII LF) in log messages.
//!
//!     ...so [`SyslogFormat::RFC5424`] should be favored whenever possible.
//!   - [`SyslogFormat::RFC5424`] implements the 2009
//!     [RFC 5424](<https://datatracker.ietf.org/doc/html/rfc5424>) standard, with
//!     proper Unicode support, parameterized logging, and nanosecond precision. Most
//!     modern syslog servers (f.ex. [syslog-ng](https://www.syslog-ng.com/products/open-source-log-management/))
//!     support it out of the box.
//!
//!     Rasant log attributes are translated to syslog parameters with a
//!     SD-ID `rasant@<attribute_index_number>`:
//!
//!       - [`Scalar`][Value::Scalar]s are encoded as string values, regardless of type.
//!       - [`List`][Value::List]s have no native syslog representation, and encode as a single
//!         string: `["val_1", "val_2", ... ]`.
//!       - [`Map`][Value::Map]s have no native syslog representation, and encode as a single
//!         string: `{"key_1": "val_1", "key_2": "val_2", ... }`.
//!   - [`SyslogFormat::RFC5424Full`] is identical to [`SyslogFormat::RFC5424`], but log
//!     attributes are also serialized as text and appended to the log message.

use ntime;
use std::io;
use std::io::{BufWriter, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, UdpSocket};
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;

use crate::attributes::{Map, Scalar, Value};
use crate::constant::{DEFAULT_LOCAL_SYSLOG_SOCKETS, HOSTNAME, NETWORK_TIMEOUT, PROCESS_ID, PROCESS_NAME};
use crate::{encoding, sink};

/// Supported syslog formats.
#[derive(Debug, PartialEq)]
pub enum SyslogFormat {
	/// [RFC 5424](https://datatracker.ietf.org/doc/html/rfc5424).
	RFC5424,
	/// [RFC 5424](https://datatracker.ietf.org/doc/html/rfc5424), with
	/// attributes rendered as part of the main message in addition to
	/// `STRUCTURED-DATA`.
	RFC5424Full,
	/// [RFC 3164](https://datatracker.ietf.org/doc/html/rfc3164). This standard
	/// was obsoleted in 2009, and [`RFC 5424`][`SyslogFormat::RFC5424`] should
	/// be favored when possible.
	RFC3164,
}

/// Supported connection types for [`Syslog`] [`sink`]s.
#[derive(Debug)]
pub enum SyslogSocket<'e> {
	/// A local syslog socket, on standard *NIX paths.
	#[cfg(unix)]
	Local,
	/// A local syslog socket for a given path.
	#[cfg(unix)]
	LocalPath(&'e str),
	/// Syslog connection over TCP with octet-counting framing
	/// ([RFC 6587](https://datatracker.ietf.org/doc/html/rfc6587) for a given address.
	TCP(&'e str),
	/// Syslog connection over TCP with transparent (i.e no) framing, for a given address. This is
	/// supported in [RFC 6587](https://datatracker.ietf.org/doc/html/rfc6587), but can cause issues
	/// with ASCII line feeds on log messages.
	TCPTransparent(&'e str),
	/// Syslog connection over UDP for a given address.
	UDP(&'e str),
	/// Dummy syslog connection, for testing.
	BlackHole(),
}

/// Configuration struct for an [`Syslog`] [`sink`].
#[derive(Debug)]
pub struct SyslogConfig<'e> {
	/// Name for this sink.
	pub name: String,
	/// Syslog server connection details.
	pub server: SyslogSocket<'e>,
	/// Syslog protocol format,
	pub format: SyslogFormat,
	/// Syslog facility code:
	pub facility: u8,
}

impl<'i> Default for SyslogConfig<'i> {
	fn default() -> Self {
		Self {
			name: String::from("default Syslog"),
			server: SyslogSocket::TCP(""),
			format: SyslogFormat::RFC5424Full,
			facility: 1, // user-level messages
		}
	}
}

impl<'i> SyslogConfig<'i> {
	#[cfg(unix)]
	/// Returns an default [`SyslogConfig`] for local syslog servers over *NIX sockets.
	pub fn default_local() -> Self {
		Self {
			name: String::from("default local Syslog"),
			server: SyslogSocket::Local,
			// most local syslogs (notably, journald) expect the older RFC 3146 format :(
			format: SyslogFormat::RFC3164,
			..Self::default()
		}
	}

	/// Returns a default [`SyslogConfig`] for syslog over UDP.
	pub fn default_udp() -> Self {
		Self {
			name: String::from("default UDP Syslog"),
			server: SyslogSocket::UDP("127.0.0.1:541"),
			..SyslogConfig::default()
		}
	}

	/// Returns a default [`SyslogConfig`] for syslog over TCP.
	/// Most syslog servers require RFC 6587 octet framing by default.
	pub fn default_tcp() -> Self {
		Self {
			name: String::from("default TCP Syslog"),
			server: SyslogSocket::TCP("127.0.0.1:601"),
			..SyslogConfig::default()
		}
	}

	/// Returns a default no-op [`SyslogConfig`].
	pub fn default_black_hole() -> Self {
		Self {
			name: String::from("default TCP Syslog"),
			server: SyslogSocket::BlackHole(),
			..SyslogConfig::default()
		}
	}
}

// Syslog writer implementation for different connection types.
#[derive(Debug)]
enum SyslogWriter {
	#[cfg(unix)]
	Datagram(UnixDatagram),
	UDP(UdpSocket, SocketAddr),
	TCPTransparent(BufWriter<TcpStream>),
	TCPFramed(BufWriter<TcpStream>),
	BlackHole(),
}

impl io::Write for SyslogWriter {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			#[cfg(unix)]
			SyslogWriter::Datagram(d) => d.send(buf),
			SyslogWriter::UDP(us, addr) => us.send_to(buf, *addr),
			SyslogWriter::TCPTransparent(ts) => {
				ts.write_all(buf)?;
				ts.write("\n".as_bytes())?;
				Ok(buf.len())
			}
			SyslogWriter::TCPFramed(ts) => {
				write!(ts, "{} ", buf.len())?;
				ts.write_all(buf)?;
				Ok(buf.len())
			}
			SyslogWriter::BlackHole() => io::empty().write(buf),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match self {
			#[cfg(unix)]
			SyslogWriter::Datagram(_) => Ok(()),
			SyslogWriter::UDP(_, _) => Ok(()),
			SyslogWriter::TCPTransparent(ts) => ts.flush(),
			SyslogWriter::TCPFramed(ts) => ts.flush(),
			SyslogWriter::BlackHole() => Ok(()),
		}
	}
}

/// A general syslog log [`sink`].
pub struct Syslog {
	name: String,
	hostname: String,
	process_name: String,
	process_id: u32,
	facility_mask: u16,
	writer: SyslogWriter,
	format: SyslogFormat,
	output_buf: Vec<u8>,
}

impl Syslog {
	/// Initializes a new [`Syslog`] log [`sink`], from a given [`SyslogConfig`].
	pub fn new(conf: SyslogConfig<'_>) -> Self {
		let hostname = HOSTNAME.clone();
		let process_name = PROCESS_NAME.clone();
		let process_id = *PROCESS_ID;
		let facility_mask = (conf.facility as u16) << 3;
		let mut errs = String::new();

		match conf.server {
			#[cfg(unix)]
			SyslogSocket::Local => {
				let dg = UnixDatagram::unbound().expect("failed to initialize Unix datagram socket for syslog");
				for path in DEFAULT_LOCAL_SYSLOG_SOCKETS {
					match dg.connect(path) {
						Ok(_) => break,
						Err(e) => errs.push_str(format!("\"{path}\": {e}").as_str()),
					};
				}
				match dg.local_addr() {
					Err(_) => panic!("failed to open a local Syslog socket: {errs}"),
					Ok(_) => Self {
						name: conf.name,
						hostname: hostname,
						process_name: process_name,
						process_id: process_id,
						facility_mask: facility_mask,
						writer: SyslogWriter::Datagram(dg),
						format: conf.format,
						output_buf: Vec::new(),
					},
				}
			}
			#[cfg(unix)]
			SyslogSocket::LocalPath(path) => {
				let dg = UnixDatagram::unbound().expect("failed to initialize Unix datagram socket for syslog");
				match dg.connect(&path) {
					Err(e) => panic!("failed to open local Syslog socket \"{path}\": {e}"),
					Ok(_) => Self {
						name: conf.name,
						hostname: hostname,
						process_name: process_name,
						process_id: process_id,
						facility_mask: facility_mask,
						writer: SyslogWriter::Datagram(dg),
						format: conf.format,
						output_buf: Vec::new(),
					},
				}
			}
			SyslogSocket::UDP(addr) => {
				let addr: SocketAddr = addr.parse().expect("invalid UDP address \"{addr}\"");
				let sock = match UdpSocket::bind("127.0.0.1:0") {
					Ok(s) => s,
					Err(e) => panic!("failed to open Syslog UDP socket to \"{addr}\": {e}"),
				};
				sock.set_write_timeout(Some(NETWORK_TIMEOUT)).expect("failed to set Syslog UDP socket timeout");
				Self {
					name: conf.name,
					hostname: hostname,
					process_name: process_name,
					process_id: process_id,
					facility_mask: facility_mask,
					writer: SyslogWriter::UDP(sock, addr),
					format: conf.format,
					output_buf: Vec::new(),
				}
			}
			SyslogSocket::TCP(addr) => {
				let addr: SocketAddr = addr.parse().expect("invalid TCP address \"{addr}\"");
				let stream = match TcpStream::connect(addr) {
					Ok(s) => s,
					Err(e) => panic!("failed to open framed Syslog TCP socket to \"{addr}\": {e}"),
				};
				stream.shutdown(Shutdown::Read).expect("failed to set framed Syslog TCP socket as write-only");
				stream.set_write_timeout(Some(NETWORK_TIMEOUT)).expect("failed to set framed Syslog TCP socket timeout");
				Self {
					name: conf.name,
					hostname: hostname,
					process_name: process_name,
					process_id: process_id,
					facility_mask: facility_mask,
					writer: SyslogWriter::TCPFramed(BufWriter::new(stream)),
					format: conf.format,
					output_buf: Vec::new(),
				}
			}
			SyslogSocket::TCPTransparent(addr) => {
				let addr: SocketAddr = addr.parse().expect("invalid TCP address \"{addr}\"");
				let stream = match TcpStream::connect(addr) {
					Ok(s) => s,
					Err(e) => panic!("failed to open Syslog TCP socket to \"{addr}\": {e}"),
				};
				stream.shutdown(Shutdown::Read).expect("failed to set Syslog TCP socket as write-only");
				stream.set_write_timeout(Some(NETWORK_TIMEOUT)).expect("failed to set Syslog TCP socket timeout");
				Self {
					name: conf.name,
					hostname: hostname,
					process_name: process_name,
					process_id: process_id,
					facility_mask: facility_mask,
					writer: SyslogWriter::TCPTransparent(BufWriter::new(stream)),
					format: conf.format,
					output_buf: Vec::new(),
				}
			}
			SyslogSocket::BlackHole() => Self {
				name: conf.name,
				hostname: hostname,
				process_name: process_name,
				process_id: process_id,
				facility_mask: facility_mask,
				writer: SyslogWriter::BlackHole(),
				format: conf.format,
				output_buf: Vec::new(),
			},
		}
	}

	// Serializes a [Scalar] as text into the write buffer for RFC 5424 messages.
	fn write_buf_scalar_5424(&mut self, attrs: &Map, s: &Scalar) -> io::Result<()> {
		let out = &mut self.output_buf;
		match s {
			Scalar::Bool(b) => write!(out, "{}", b),
			Scalar::String(s, _) => encoding::str_write(out, s.as_str(), &encoding::Mode::Utf8Rfc5424ParamValue),
			Scalar::StringSlice(s, _) => encoding::str_write(out, s, &encoding::Mode::Utf8Rfc5424ParamValue),
			Scalar::StringIndex(idx, _) => encoding::str_write(out, attrs.str_by_idx(*idx), &encoding::Mode::Utf8Rfc5424ParamValue),
			Scalar::Int(i) => write!(out, "{}", i),
			Scalar::LongInt(i) => write!(out, "{}", i),
			Scalar::Size(s) => write!(out, "{}", s),
			Scalar::Uint(i) => write!(out, "{}", i),
			Scalar::LongUint(u) => write!(out, "{}", u),
			Scalar::Usize(u) => write!(out, "{}", u),
			Scalar::Float(f) => write!(out, "{}", f),
		}
	}

	// Serializes a [Value] as text into the write buffer for RFC 5424 messages.
	fn write_buf_value_5424(&mut self, attrs: &Map, v: &Value) -> io::Result<()> {
		match v {
			Value::Scalar(s) => {
				// scalars are always encoded as strings: "scalar_value"
				self.output_buf.write("\"".as_bytes())?;
				self.write_buf_scalar_5424(attrs, s)?;
				self.output_buf.write("\"".as_bytes())?;
			}
			Value::List(ss) => {
				// lists are encoded as a single string: "[\"val_1\", \"val_2\", ... \]".
				// note the escaping on the final bracket!
				self.output_buf.write("\"[".as_bytes())?;
				for i in 0..ss.len() {
					if i != 0 {
						self.output_buf.write(", ".as_bytes())?;
					}
					self.output_buf.write("\\\"".as_bytes())?;
					self.write_buf_scalar_5424(attrs, &ss[i])?;
					self.output_buf.write("\\\"".as_bytes())?;
				}
				self.output_buf.write("\\]\"".as_bytes())?;
			}
			Value::Map(mkeys, mvals) => {
				// maps are encoded as a single string: "{\"key_1\": \"val_1\", ... }"
				self.output_buf.write("\"{".as_bytes())?;
				for i in 0..mkeys.len() {
					if i != 0 {
						self.output_buf.write(", ".as_bytes())?;
					}
					self.output_buf.write("\\\"".as_bytes())?;
					self.write_buf_scalar_5424(attrs, &mkeys[i])?;
					self.output_buf.write("\\\": \\\"".as_bytes())?;
					self.write_buf_scalar_5424(attrs, &mvals[i])?;
					self.output_buf.write("\\\"".as_bytes())?;
				}
				self.output_buf.write("}\"".as_bytes())?;
			}
		}

		Ok(())
	}

	// Serializes all attributes into the write buffer for RFC 5424 messages.
	fn write_buf_attributes_5424(&mut self, attrs: &Map) -> io::Result<()> {
		if attrs.len() == 0 {
			self.output_buf.write("-".as_bytes())?;
			return Ok(());
		}

		// "[rasant@0 error=\"timeout reading from socket\" items=1120213 done=3493 extra=\"\\[1, 2, 3, 4, 5\\]\" more_extra=\"{{\\\"lala\\\": 1, \\\"lele\\\": 2}}\"]"
		let mut i: usize = 0;
		for (key, val) in attrs.iter() {
			write!(&mut self.output_buf, "[rasant@{i} ")?;
			encoding::str_write(&mut self.output_buf, key, &encoding::Mode::Utf8)?;
			self.output_buf.write("=".as_bytes())?;
			self.write_buf_value_5424(attrs, &val)?;
			self.output_buf.write("]".as_bytes())?;
			i += 1
		}

		Ok(())
	}

	// Serializes all attributes as text into the write buffer.
	// These are rendered in plaintext, as part of the message, as older syslog versions
	// have no support for structured data.
	fn write_buf_attributes_text(&mut self, attrs: &Map) -> io::Result<()> {
		// TODO: handle escaping?
		for (key, val) in attrs.iter() {
			write!(&mut self.output_buf, " {key}={val}")?;
		}

		Ok(())
	}
}

impl sink::Sink for Syslog {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &Map) -> io::Result<()> {
		self.output_buf.clear();
		match self.format {
			SyslogFormat::RFC5424 | SyslogFormat::RFC5424Full => {
				write!(&mut self.output_buf, "<{pri}>1 ", pri = self.facility_mask + update.level.syslog_severity())?;
				update.when.write(&mut self.output_buf, &ntime::Format::UtcNanosRFC3339)?;
				write!(
					self.output_buf,
					" {hostname} {process_name} {process_id} - ",
					hostname = self.hostname,
					process_name = self.process_name,
					process_id = self.process_id
				)?;
				self.write_buf_attributes_5424(attrs)?;
				self.output_buf.write(&[b' '])?;
				encoding::str_write(&mut self.output_buf, update.msg.as_str(), &encoding::Mode::Utf8Bom)?;
				if self.format == SyslogFormat::RFC5424Full {
					self.write_buf_attributes_text(attrs)?;
				}
			}
			SyslogFormat::RFC3164 => {
				write!(&mut self.output_buf, "<{pri}>", pri = self.facility_mask + update.level.syslog_severity())?;
				update.when.write(&mut self.output_buf, &ntime::Format::LocalRFC3164)?;
				write!(
					self.output_buf,
					" {process_name}[{process_id}]: {message}",
					process_name = self.process_name,
					process_id = self.process_id,
					message = update.msg,
				)?;
				self.write_buf_attributes_text(attrs)?;
			}
		}

		self.writer.write(&self.output_buf)?;
		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		self.writer.flush()
	}
}

impl Drop for Syslog {
	fn drop(&mut self) {
		if let Err(e) = self.writer.flush() {
			panic!("failed to flush sink {name} on drop(): {e}", name = &self.name);
		}
	}
}

#[cfg(unix)]
/// Returns an intitalized [`Syslog`] log [`sink`], with defaults for local syslog servers.
pub fn local() -> Syslog {
	Syslog::new(SyslogConfig::default_local())
}

#[cfg(unix)]
/// Returns an intitalized [`Syslog`] log [`sink`], with defaults for local syslog servers over UDP.
pub fn local_udp() -> Syslog {
	Syslog::new(SyslogConfig::default_udp())
}

#[cfg(unix)]
/// Returns an intitalized [`Syslog`] log [`sink`], with defaults for local syslog servers over TCP.
pub fn local_tcp() -> Syslog {
	Syslog::new(SyslogConfig::default_tcp())
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;

	use ntime::Timestamp;
	use std::str;

	use crate::attributes::{Scalar, Value};
	use crate::level::Level;
	use crate::sink::{LogUpdate, Sink};

	#[test]
	fn output_format() {
		for tc in [
			(
				SyslogFormat::RFC3164,
				"<12>Apr 12 19:56:39 test_process[1234]: test Syslog message update ❤\u{fe0f} an_int=123 a_float=-456.789 some_string=\"hi there!\" a_list=[0x14da0eb6, true] a_map={\"key #1\": false, \"key #2\": \"weee\"}",
			),
			(
				SyslogFormat::RFC5424,
				"<12>1 2026-04-12T17:56:39.123000456Z localhost test_process 1234 - [rasant@0 an_int=\"123\"][rasant@1 a_float=\"-456.789\"][rasant@2 some_string=\"hi there!\"][rasant@3 a_list=\"[\\\"349834934\\\", \\\"true\\\"\\]\"][rasant@4 a_map=\"{\\\"key #1\\\": \\\"false\\\", \\\"key #2\\\": \\\"weee\\\"}\"] \u{feff}test Syslog message update ❤\u{fe0f}",
			),
			(
				SyslogFormat::RFC5424Full,
				"<12>1 2026-04-12T17:56:39.123000456Z localhost test_process 1234 - [rasant@0 an_int=\"123\"][rasant@1 a_float=\"-456.789\"][rasant@2 some_string=\"hi there!\"][rasant@3 a_list=\"[\\\"349834934\\\", \\\"true\\\"\\]\"][rasant@4 a_map=\"{\\\"key #1\\\": \\\"false\\\", \\\"key #2\\\": \\\"weee\\\"}\"] \u{feff}test Syslog message update ❤\u{fe0f} an_int=123 a_float=-456.789 some_string=\"hi there!\" a_list=[0x14da0eb6, true] a_map={\"key #1\": false, \"key #2\": \"weee\"}",
			),
		] {
			let (format, want) = tc;

			let update = LogUpdate::new(
				Timestamp::from_utc_date(2026, 04, 12, 17, 56, 39, 123, 456).expect("failed to initialize timestamp"),
				Level::Warning,
				"test Syslog message update ❤️".into(),
			);

			let mut attrs = Map::new();
			attrs.insert("an_int", Value::from(123 as i32));
			attrs.insert("a_float", Value::from(-456.789));
			attrs.insert("some_string", Value::from("hi there!"));
			attrs.insert("a_list", Value::from(&[Scalar::from(349834934 as usize), Scalar::from(true)]));
			attrs.insert("a_map", Value::from((&[Scalar::from("key #1"), Scalar::from("key #2")], &[Scalar::from(false), Scalar::from("weee")])));

			let mut sink = Syslog::new(SyslogConfig {
				server: SyslogSocket::BlackHole(),
				format: format,
				..SyslogConfig::default()
			});
			sink.process_name = "test_process".into();
			sink.process_id = 1234;
			assert!(sink.log(&update, &attrs).is_ok());

			let got = str::from_utf8(&sink.output_buf).unwrap();

			assert_eq!(got, want);
		}
	}
}
