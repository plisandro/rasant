use ntime::Duration;
use std::env;
use std::process;
use std::sync::LazyLock;

use crate::c_bindings;

/// Ennvironment variable to detect the presence of ANSI color-capable terminals.
pub static ENV_VAR_COLORTERM: &str = "COLORTERM";

/// Process ID running this module.
pub static PROCESS_ID: LazyLock<u32> = LazyLock::new(|| process::id());

/// Name of the process running this module.
pub static PROCESS_NAME: LazyLock<String> = LazyLock::new(|| {
	let current_exe = env::current_exe();
	match &current_exe {
		Ok(ce) => match ce.file_name() {
			Some(n) => match n.to_str() {
				Some(s) => return String::from(s),
				None => String::from("process_invalid_name"),
			},
			_ => String::from("process_no_name"),
		},
		_ => String::from("process"),
	}
});

/// System hostname
// TODO: Replace with std::net::hostname(), once no longer experimental.
pub static HOSTNAME: LazyLock<String> = LazyLock::new(|| match c_bindings::c_get_hostname() {
	Some(s) => s,
	None => String::from("localhost"),
});

/// UTF-8 byte-order-mark
pub static UTF8_BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
/// Maximum size of a UTF-8 encoded char, in bytes.
pub const UTF_8_CHAR_MAX_SIZE: usize = 4;

/// Attribute key for error details.
pub const ATTRIBUTE_KEY_ERROR: &str = "error";
/// Attribute key for log level.
pub const ATTRIBUTE_KEY_LEVEL: &str = "level";
/// Attribute key for log messages.
pub const ATTRIBUTE_KEY_MESSAGE: &str = "message";
/// Attribute key for timestamps, as string.
pub const ATTRIBUTE_KEY_TIME: &str = "time";
/// Attribute key for numeric timestamps;
pub const ATTRIBUTE_KEY_TIMESTAMP: &str = "timestamp";
/// Attribute key for logger IDs.
pub const ATTRIBUTE_KEY_LOGGER_ID: &str = "logger_id";

/// Restricted attribute keys; these cannot be set by end users.
pub const ATTRIBUTE_KEYS_RESTRICTED: [&str; 3] = [ATTRIBUTE_KEY_LEVEL, ATTRIBUTE_KEY_TIME, ATTRIBUTE_KEY_TIMESTAMP];
/// Priority attribute keys. These are always returned first when iterating through attributes.
pub const ATTRIBUTE_KEYS_PRIORITY: [&str; 2] = [ATTRIBUTE_KEY_MESSAGE, ATTRIBUTE_KEY_ERROR];

/// Maximum allowed [`crate::logger::Logger`] depth.
pub const MAX_LOGGER_DEPTH: u16 = 1024;

/// Default log separator for binary format outputs.
pub const DEFAULT_LOG_DELIMITER_BINARY: &[u8] = "".as_bytes();

/// Default log separator for string format outputs.
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_LOG_DELIMITER_STRING: &[u8] = "\n".as_bytes();
#[cfg(target_os = "windows")]
pub const DEFAULT_LOG_DELIMITER_STRING: &[u8] = "\r\n".as_bytes();

/// How long to wait for open threads to finalize.
pub const THREAD_FINALIZE_TIMEOUT: Duration = Duration::from_secs(30);
/// How often to check on open threads for finalization.
pub const THREAD_FINALIZE_SPINLOCK_WAIT: Duration = Duration::from_millis(100);

/// Timeout for network operations.
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);
/// Default journald *NIX socket for writes
pub const DEFUALT_JOURNALD_SOCKET: &str = "/run/systemd/journal/socket";
/// Default local *NIX syslog sockets.
#[cfg(unix)]
pub const DEFAULT_LOCAL_SYSLOG_SOCKETS: [&str; 3] = ["/dev/log", "/var/run/log", "/var/run/syslog"];
