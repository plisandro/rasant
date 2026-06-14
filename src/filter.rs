//! Log filters for Rasant [logger][crate::logger::Logger] instances.
//!
//! This module defines the [`Filter`] traits for filter, and
//! exports all available filter types.

pub mod level;
pub mod matches;
pub mod sample;

use crate::sink::LogUpdate;

/// Defines a log filter usable by [Logger][`crate::logger::Logger`]s.
pub trait Filter {
	/// Returns a [`&str`] name for the filter.
	fn name(&self) -> &str;
	/// Verifies whether a [`LogUpdate`] with attributes shouuld be logged.
	fn pass<'f>(&mut self, update: &'f LogUpdate) -> bool;

	/// Verifies whether a [`LogUpdate`] with attributes shouuld be skipped.
	fn skip<'f>(&mut self, update: &LogUpdate) -> bool {
		!self.pass(update)
	}
}
