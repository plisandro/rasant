//! Sampling [`filter`] module.
//!
//! Sampling filters a subset of all log updates, regardless of content,
//! and are intended for monitoring and/or statistical analysis.

use ntime;
use ntime::{Duration, Timestamp};
use std::string;
use std::u64;

use crate::attributes;
use crate::filter;
use crate::sink;
use crate::types::Rand;

/// Configuration struct for a [`Random`]s sampling [filter][`filter::Filter`].
pub struct RandomConfig {
	/// Probability of log updates being selected, between `0.0` and `1.0`.
	pub probability: f32,
}

/// A random sampling [filter][`filter::Filter`], selecting log operations by probability.
pub struct Random {
	name: string::String,
	rand: Rand,
	threshold: u64,
}

impl Random {
	/// Initializes a new [`Random`] sampling log [filter][`filter::Filter`], from a given [`RandomConfig`].
	pub fn new(conf: RandomConfig) -> Self {
		let threshold: u64;
		if conf.probability <= 0_f32 {
			threshold = 0;
		} else if conf.probability >= 1_f32 {
			threshold = u64::MAX;
		} else {
			threshold = (u64::MAX / 1000000) * (1000000_f32 * conf.probability) as u64;
		}

		Self {
			name: format!("random sample filter ({prob:.02}%)", prob = 100_f32 * conf.probability),
			rand: Rand::new(),
			threshold: threshold,
		}
	}

	/// A [`Random`] sampling [filter][`filter::Filter`] with a fixed random seed, used only for testing.
	fn with_seed(conf: RandomConfig, seed: u64) -> Self {
		let mut f = Self::new(conf);
		f.rand = Rand::with_seed(seed);

		f
	}
}

impl filter::Filter for Random {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, _: &attributes::Map) -> bool {
		self.rand.next() <= self.threshold
	}
}

/// Configuration struct for a [`Step`]s sampling [filter][`filter::Filter`].
pub struct StepConfig {
	/// Interval between log updates.
	pub step: u64,
}

/// A basic step sampling [filter][`filter::Filter`], selecting only every N-th log operation.
pub struct Step {
	name: string::String,
	step: u64,
	count: u64,
}

impl Step {
	/// Initializes a new [`Step`] sampling log [filter][`filter::Filter`], from a given [`StepConfig`].
	pub fn new(conf: StepConfig) -> Self {
		Self {
			name: format!("step={step} sample filter", step = conf.step),
			step: conf.step,
			count: 0,
		}
	}
}

impl filter::Filter for Step {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, _: &attributes::Map) -> bool {
		if self.step == 0 {
			return true;
		}

		if self.count >= (self.step - 1) {
			self.count = 0;
			return true;
		}

		self.count += 1;
		false
	}
}

/// Configuration struct for a [`RandomStep`] sampling [filter][`filter::Filter`].
pub struct RandomStepConfig {
	/// Interval size from which one log update gets randomly selected.
	pub step: u64,
}

/// A random sampling [filter][`filter::Filter`], selecting one out of every N log events, at random.
pub struct RandomStep {
	name: string::String,
	step: u64,
	count: u64,
	target: u64,
	rand: Rand,
}

impl RandomStep {
	/// Initializes a new [`RandomStep`] sampling log [filter][`filter::Filter`], from a given [`RandomStepConfig`].
	pub fn new(conf: RandomStepConfig) -> Self {
		Self {
			name: format!("step={step} sample filter", step = conf.step),
			step: conf.step,
			count: 0,
			target: 0,
			rand: Rand::new(),
		}
	}

	/// A [`RandomStep`] sampling [filter][`filter::Filter`] with a fixed random seed, used only for testing.
	fn with_seed(conf: RandomStepConfig, seed: u64) -> Self {
		let mut f = Self::new(conf);
		f.rand = Rand::with_seed(seed);

		f
	}
}

impl filter::Filter for RandomStep {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, _: &attributes::Map) -> bool {
		if self.step == 0 {
			return true;
		}
		let res = self.count == self.target;

		if self.count >= (self.step - 1) {
			self.count = 0;
			self.target = self.rand.next() % self.step;
		} else {
			self.count += 1;
		}

		return res;
	}
}

/// Configuration struct for a [`Burst`] sampling [filter][`filter::Filter`].
pub struct BurstConfig {
	/// Burst period.
	pub period: ntime::Duration,
	/// Maximum number of log updates allowed per burst period.
	pub max_updates: u64,
}

/// A burst sampling [filter][`filter::Filter`], selecting a given maximum
/// of log updates over a given time period.
pub struct Burst {
	name: string::String,
	period: Duration,
	period_end: Timestamp,
	period_count: u64,
	max_updates: u64,
}

impl Burst {
	/// Initializes a new [`Burst`] sampling log [`filter`], from a given [`BurstConfig`].
	pub fn new(conf: BurstConfig) -> Self {
		Self {
			name: format!("burst sample filter (max {max} per {period:?})", max = conf.max_updates, period = conf.period),
			period: conf.period,
			period_end: ntime::Timestamp::epoch(),
			period_count: 0,
			max_updates: conf.max_updates,
		}
	}
}

impl filter::Filter for Burst {
	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn pass(&mut self, _: &sink::LogUpdate, _: &attributes::Map) -> bool {
		if self.period == Duration::from_millis(0) {
			return true;
		}

		let now = Timestamp::now();
		if now >= self.period_end {
			self.period_end = now;
			self.period_end.add_duration(&self.period);
			self.period_count = 0;
		}

		if self.period_count >= self.max_updates {
			return false;
		}

		self.period_count += 1;
		true
	}
}
/* ----------------------- Tests ----------------------- */

#[cfg(test)]
mod random {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;
	use crate::level::Level;

	#[test]
	fn filtering() {
		let mut filter = Random::with_seed(RandomConfig { probability: 0.33 }, 27182818);
		let args = attributes::Map::new();
		let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "this is a test log".into());

		let mut got: Vec<usize> = Vec::new();
		for i in 0..50 {
			if filter.pass(&update, &args) {
				got.push(i + 1);
			}
		}

		let want: Vec<usize> = [1, 10, 11, 13, 18, 23, 28, 31, 32, 34, 35, 36, 46, 50].to_vec();
		assert_eq!(got, want);
	}
}

#[cfg(test)]
mod step {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;
	use crate::level::Level;

	#[test]
	fn filtering() {
		let mut filter = Step::new(StepConfig { step: 3 });
		let args = attributes::Map::new();
		let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "this is a test log".into());

		let mut got: Vec<usize> = Vec::new();
		for i in 0..15 {
			if filter.pass(&update, &args) {
				got.push(i + 1);
			}
		}

		let want: Vec<usize> = [3, 6, 9, 12, 15].to_vec();
		assert_eq!(got, want);
	}
}

#[cfg(test)]
mod random_step {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;
	use crate::level::Level;

	#[test]
	fn filtering() {
		let mut filter = RandomStep::with_seed(RandomStepConfig { step: 7 }, 27182818);
		let args = attributes::Map::new();
		let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "this is a test log".into());

		let mut got: Vec<usize> = Vec::new();
		for i in 0..50 {
			if filter.pass(&update, &args) {
				got.push(i + 1);
			}
		}

		let want: Vec<usize> = [1, 11, 20, 23, 33, 36, 46].to_vec();
		assert_eq!(got, want);
	}
}

#[cfg(test)]
mod burst {
	use super::*;
	use ntime::Timestamp;

	use crate::filter::Filter;
	use crate::level::Level;

	#[test]
	fn filtering() {
		let args = attributes::Map::new();
		let update = sink::LogUpdate::new(Timestamp::now(), Level::Info, "this is a test log".into());
		let mut filter = Burst::new(BurstConfig {
			period: Duration::from_millis(5),
			max_updates: 3,
		});

		let mut got: Vec<usize> = Vec::new();
		for i in 0..15 {
			if filter.pass(&update, &args) {
				got.push(i + 1);
			}
			ntime::sleep_millis(1);
		}

		let want: Vec<usize> = [1, 2, 3, 6, 7, 8, 11, 12, 13].to_vec();
		assert_eq!(got, want);
	}
}
