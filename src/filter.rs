//! Log filters for use with Rasant [Logger][`crate::logger::Logger`] instances.
//!
//! This module defines the [`Filter`] traits for filter, and
//! exports all available filter types.

pub mod levels;
pub mod matches;

use crate::attributes;
use crate::sink::LogUpdate;

/// Defines a log filter usable by [Logger][`crate::logger::Logger`]s.
pub trait Filter {
	/// Returns a [`&str`] name for the filter.
	fn name(&self) -> &str;
	/// Verifies whether a [`LogUpdate`] with attributes shouuld be logged.
	fn pass(&mut self, update: &LogUpdate, attrs: &attributes::Map) -> bool;

	/// Verifies whether a [`LogUpdate`] with attributes shouuld be skipped.
	fn skip(&mut self, update: &LogUpdate, attrs: &attributes::Map) -> bool {
		!self.pass(update, attrs)
	}
}
