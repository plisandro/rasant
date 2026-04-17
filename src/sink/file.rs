//! Generic file logging log [sink][`crate::sink`] module.
//!
//! Built around [`IO`], so it inherits pretty much all of
//! its configuration options.
use std::fs;
//use std::io::Write;
use std::path::PathBuf;

use crate::format;
use crate::sink::io::{IO, IOConfig};

/// Configuration struct for a file [`IO`] log sink.
pub struct FileConfig {
	/// Name for this sink.
	pub name: String,
	/// Output formatting configuration.
	pub formatter_cfg: format::FormatterConfig,
	/// Whether file writes are buffered or not.
	pub buffered: bool,
	/// Whether to flush immediately after every write operation.
	pub flush_on_write: bool,
	/// Wheter to append on existing file paths, or truncate them.
	pub append: bool,
	/// Full file path to write to, as a [`std::path::PathBuf`].
	pub path: Option<PathBuf>,
}

impl Default for FileConfig {
	fn default() -> Self {
		Self {
			name: "file".into(),
			formatter_cfg: format::FormatterConfig::default(),
			buffered: true,
			flush_on_write: false,
			append: true,
			path: None,
		}
	}
}

/// Initializes a file log [sink][`crate::sink`] from a [`FileConfig`].
pub fn new<'f>(conf: FileConfig) -> IO<'f> {
	let Some(path) = conf.path else {
		panic!("missing path for logfile sink");
	};

	let exists = match fs::exists(&path) {
		Ok(b) => b,
		Err(e) => panic!("failed to check if log file for \"{name}\" at {path} exists: {e}", name = &conf.name, path = path.display()),
	};
	let fh = match fs::File::options().create(true).write(true).append(conf.append).truncate(!conf.append).open(&path) {
		Ok(fh) => fh,
		Err(e) => {
			panic!("failed to open log file for \"{name}\" at {path}: {e:?}", name = &conf.name, path = path.display());
		}
	};

	IO::new(IOConfig {
		name: conf.name,
		formatter_cfg: conf.formatter_cfg,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		out: Some(fh),
		initial_delimiter: exists,
		..IOConfig::default()
	})
}
