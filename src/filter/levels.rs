//! Level [`filter`] module.
//!
//! Allows to filter log updates by level.
//!
//! Note that [logger][`crate::logger::Logger`]s will still apply level checks, so consider
//! enabling [set_all_levels()][`crate::logger::Logger::set_all_levels`] when using this filter.

// TODO: rename module to just level.rs, and "Levels" to "In".

use std::string;

use crate::attributes;
use crate::filter;
use crate::level::Level;
use crate::sink;

/// Configuration struct for a [`Level`]s [`filter`].
pub struct LevelsConfig<const N: usize> {
	/// [`Level`]s to allow logging by.
	pub levels: [Level; N],
}

/// A [`Level`]s [filter][`filter::Filter`].
pub struct Levels {
	name: string::String,
	levels: Vec<Level>,
}

impl Levels {
	/// Initializes a new [`Levels`] log [`filter`], from a given [`LevelsConfig`].
	pub fn new<const N: usize>(conf: LevelsConfig<N>) -> Self {
		Self {
			name: format!("level filter for {levels:?}", levels = conf.levels),
			levels: conf.levels.to_vec(),
		}
	}
}

impl filter::Filter for Levels {
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
	fn filtering() {
		let args = attributes::Map::new();

		let mut filter = Levels::new(LevelsConfig {
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
