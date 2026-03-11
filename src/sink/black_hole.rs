use std::string;

use crate::sink;
use crate::sink::format;

pub struct BlackHoleConfig {
	pub formatter_cfg: format::FormatterConfig,
}

impl Default for BlackHoleConfig {
	fn default() -> Self {
		Self {
			formatter_cfg: format::FormatterConfig {
				output: format::OutputFormat::Compact,
				..format::FormatterConfig::default()
			},
		}
	}
}

// NULL log sync for testing.
pub struct BlackHole {
	name: string::String,
	formatter: format::Formatter,
}

impl BlackHole {
	pub fn new(conf: BlackHoleConfig) -> Self {
		Self {
			name: "black hole NULL logger".into(),
			formatter: format::Formatter::new(conf.formatter_cfg),
		}
	}
}

impl sink::Sink for BlackHole {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn write(&mut self, update: &sink::LogUpdate) {
		let out = self.formatter.format(&update);
		drop(out);
	}

	fn flush(&mut self) {}

	fn drop(&self) {}
}

pub fn default() -> BlackHole {
	BlackHole::new(BlackHoleConfig::default())
}
