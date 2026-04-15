use ntime::Duration;

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

/// Maximum size for a [ShortString][`types::ShortString`], in bytes.
pub const SHORT_STRING_MAX_SIZE: usize = 32;

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
pub const THREAD_FINALIZE_TIMEOUT: Duration = Duration::from_secs(5);
/// How often to check on open threads for finalization.
pub const THREAD_FINALIZE_SPINLOCK_WAIT: Duration = Duration::from_millis(50);
