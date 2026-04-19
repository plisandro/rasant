//! Level [`filter`] module.
//!
//! Allows to filter log updates by level.
//!
//! Note that [logger][`crate::logger::Logger`]s will still apply level checks, so consider
//! enabling [set_all_levels()][`crate::logger::Logger::set_all_levels`] when using filters
//! in this module.

use std::string;

use crate::attributes;
use crate::filter;
use crate::level::Level;
use crate::sink;

/// Configuration struct for a [`In`] level [`filter`].
pub struct InConfig<const N: usize> {
	/// [`Level`]s to allow logging for.
	pub levels: [Level; N],
}

/// A level [filter][`filter::Filter`] which selects log operations
/// if they match any of the provided [`Level`]s.
pub struct In {
	name: string::String,
	levels: Vec<Level>,
}

impl In {
	/// Initializes a new [`In`] level [`filter`], from a given [`InConfig`].
	pub fn new<const N: usize>(conf: InConfig<N>) -> Self {
		Self {
			name: format!("level filter for {levels:?}", levels = conf.levels),
			levels: conf.levels.to_vec(),
		}
	}
}

impl filter::Filter for In {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, update: &sink::LogUpdate, _: &attributes::Map) -> bool {
		self.levels.iter().any(|l| *l == update.level)
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;

	#[test]
	fn level_in() {
		let args = attributes::Map::new();

		let mut filter = In::new(InConfig {
			levels: [Level::Trace, Level::Warning, Level::Panic],
		});

		for tc in [
			(Level::Trace, true),
			(Level::Debug, false),
			(Level::Info, false),
			(Level::Warning, true),
			(Level::Error, false),
			(Level::Fatal, false),
			(Level::Panic, true),
		] {
			let (level, want): (Level, bool) = tc;
			let update = sink::LogUpdate::new(Timestamp::now(), level, "this is a test log".into());

			assert_eq!(filter.pass(&update, &args), want);
		}
	}
}
