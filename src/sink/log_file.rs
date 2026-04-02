use ntime;
use std::env;
use std::path;
use std::process;

use crate::sink::file;
use crate::sink::format;
use crate::sink::io::IO;

pub struct LogFileConfig {
	pub log_directory: path::PathBuf,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
	pub append: bool,
	pub file_path: Option<String>,
}

impl Default for LogFileConfig {
	fn default() -> Self {
		Self {
			log_directory: env::temp_dir(),
			formatter_cfg: format::FormatterConfig::default(),
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
			append: true,
			file_path: None,
		}
	}
}

pub fn new<'f>(conf: LogFileConfig) -> IO<'f> {
	// TODO: resolve process name
	let process_name = "process";
	let log_file_name = path::PathBuf::from(format!(
		"{process_name}_{time}_{pid}.log",
		process_name = process_name,
		// TODO: change to local
		time = ntime::Timestamp::now().as_string(&ntime::Format::UtcFileName),
		pid = process::id(),
	));

	let mut log_path = conf.log_directory;
	log_path.push(log_file_name);

	file::new(file::FileConfig {
		name: format!("log file for {process_name}"),
		path: Some(log_path),
		formatter_cfg: conf.formatter_cfg,
		delimiter: conf.delimiter,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		append: conf.append,
	})
}

pub fn default<'f>() -> IO<'f> {
	new(LogFileConfig::default())
}

pub fn default_json<'f>() -> IO<'f> {
	new(LogFileConfig {
		formatter_cfg: format::FormatterConfig {
			format: format::OutputFormat::Json,
			..format::FormatterConfig::default()
		},
		..LogFileConfig::default()
	})
}
