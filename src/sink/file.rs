use std::fs::File;
use std::path::PathBuf;

use crate::sink::format;
use crate::sink::io::{IO, IOConfig};

pub struct FileConfig {
	pub name: String,
	pub formatter_cfg: format::FormatterConfig,
	pub delimiter: String,
	pub buffered: bool,
	pub flush_on_write: bool,
	pub append: bool,
	pub path: Option<PathBuf>,
}

impl Default for FileConfig {
	fn default() -> Self {
		Self {
			name: "file".into(),
			formatter_cfg: format::FormatterConfig::default(),
			delimiter: "\n".into(),
			buffered: true,
			flush_on_write: false,
			append: true,
			path: None,
		}
	}
}

pub fn new(conf: FileConfig) -> IO {
	let Some(path) = conf.path else {
		panic!("missing path for logfile sink");
	};

	let fh = match File::options().create(true).write(true).append(conf.append).truncate(!conf.append).open(&path) {
		Ok(fh) => fh,
		Err(e) => {
			panic!("failed to open log file for \"{name}\" at {path}: {e:?}", name = &conf.name, path = path.display(), e = e);
		}
	};

	IO::new(IOConfig {
		name: conf.name,
		formatter_cfg: conf.formatter_cfg,
		delimiter: conf.delimiter,
		buffered: conf.buffered,
		flush_on_write: conf.flush_on_write,
		out: Some(fh),
	})
}
