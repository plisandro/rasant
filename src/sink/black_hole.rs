use std::io;
use std::string;

use crate::attributes;
use crate::sink;
use crate::sink::format;

pub struct BlackHoleConfig {
	pub formatter_cfg: format::FormatterConfig,
}

impl Default for BlackHoleConfig {
	fn default() -> Self {
		Self {
			formatter_cfg: format::FormatterConfig {
				format: format::OutputFormat::Compact,
				..format::FormatterConfig::default()
			},
		}
	}
}

// NULL log sync for testing.
pub struct BlackHole {
	name: string::String,
	formatter: format::Formatter,
	out: io::Empty,
}

impl BlackHole {
	pub fn new(conf: BlackHoleConfig) -> Self {
		Self {
			name: "black hole NULL logger".into(),
			formatter: format::Formatter::new(conf.formatter_cfg),
			out: io::empty(),
		}
	}
}

impl sink::Sink for BlackHole {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn log(&mut self, update: &sink::LogUpdate, attrs: &attributes::Map) -> io::Result<()> {
		self.formatter.write(&mut self.out, update, attrs)
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

pub fn default() -> BlackHole {
	BlackHole::new(BlackHoleConfig::default())
}
