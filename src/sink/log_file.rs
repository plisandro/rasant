//! Log file [sink][`crate::sink::Sink`] module.
//!
//! Log file sinks are very similar to regular [`mod@file`] sinks, but impose an
//! opinionated file name format. Only logging directories are configurable.
use ntime;
use std::env;
use std::path;
use std::process;

use crate::format;
use crate::sink::file;
use crate::sink::io::IO;

/// Configuration struct for an [`IO`] log file [sink][`crate::sink::Sink`].
pub struct LogFileConfig {
	/// Base directory for log files, as a [`std::path::PathBuf`]
	pub log_directory: path::PathBuf,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// String delimiter, inserted between log writes.
	pub buffered: bool,
	/// Whether to flush immediately after every write operation.
	pub flush_on_write: bool,
	/// Wheter to append on existing file paths, or truncate them.
	pub append: bool,
}

impl<'i> Default for LogFileConfig {
	fn default() -> Self {
		Self {
			log_directory: env::temp_dir(),
			formatter_cfg: format::FormatterConfig::default(),
			buffered: true,
			flush_on_write: false,
			append: true,
		}
	}
}

/// Initializes a [`IO`] log file [sink][`crate::sink::Sink`] from a [`LogFileConfig`].
pub fn new<'f>(conf: LogFileConfig) -> IO<'f> {
	// TODO: resolve process name
	let process_name = "process";
	let log_file_name = path::PathBuf::from(format!(
		"{process_name}_{time}_{pid}.log",
		process_name = process_name,
		// TODO: change to local?
		time = ntime::Timestamp::now().as_string(&ntime::Format::UtcFileName),
		pid = process::id(),
	));

	let mut log_path = conf.log_directory;
	log_path.push(log_file_name);

	file::new(file::FileConfig {
		name: format!("log file for {process_name}"),
		path: Some(log_path),
		formatter_cfg: conf.formatter_cfg,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		append: conf.append,
	})
}

/// Returns an initialized log file [sink][`crate::sink::Sink`] for text, with default values.
pub fn default<'f>() -> IO<'f> {
	new(LogFileConfig::default())
}

/// Returns an initialized log file [sink][`crate::sink::Sink`] for JSON, with default values.
pub fn default_json<'f>() -> IO<'f> {
	new(LogFileConfig {
		formatter_cfg: format::FormatterConfig {
			format: format::OutputFormat::Json,
			..format::FormatterConfig::default()
		},
		..LogFileConfig::default()
	})
}
