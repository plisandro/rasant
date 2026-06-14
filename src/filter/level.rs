//! Level [filter][Filter] module.
//!
//! Allows to filter log updates by level.
//!
//! Note that [logger][crate::logger::Logger]s will still apply level checks, so consider
//! enabling [`Logger::set_all_levels()`][crate::logger::Logger::set_all_levels] when using filters
//! in this module.

use std::string;

use crate::filter::Filter;
use crate::level::Level;
use crate::sink;

/// Configuration struct for a [`In`] level [`Filter`].
pub struct InConfig<const N: usize> {
	/// [`Level`]s to allow logging for.
	pub levels: [Level; N],
}

/// A level [`Filter`] which selects log operations
/// if they match any of the provided [`Level`]s.
pub struct In {
	name: string::String,
	levels: Vec<Level>,
}

impl In {
	/// Initializes a new [`In`] level [`Filter`], from a given [`InConfig`].
	pub fn new<const N: usize>(conf: InConfig<N>) -> Self {
		Self {
			name: format!("level filter for {levels:?}", levels = conf.levels),
			levels: conf.levels.to_vec(),
		}
	}
}

impl Filter for In {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass<'f>(&mut self, update: &sink::LogUpdate) -> bool {
		self.levels.iter().any(|l| *l == *update.level())
	}
}

/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod tests {
	use super::*;
	use ntime::Timestamp;

	use crate::attributes;
	use crate::filter::Filter;
	use crate::sink::LogUpdate;

	#[test]
	fn level_in() {
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
			let pupdate = sink::PartialLogUpdate::new(Timestamp::now(), level, "this is a test log".into());

			assert_eq!(filter.pass(&LogUpdate::from((&pupdate, &attributes::Map::new()))), want);
		}
	}
}
