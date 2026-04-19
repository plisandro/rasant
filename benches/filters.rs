use divan::{Bencher, counter};
use rasant as r;
use rasant::filter;
use rasant::sink::black_hole;
use rasant::{FormatterConfig, Level, Logger, OutputFormat, TimeFormat};

const BENCHMARK_LOG_ITEMS: usize = 10000;

fn main() {
	divan::main();
}

fn init_logger() -> Logger {
	let mut log = rasant::Logger::new();
	log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::Compact,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..black_hole::BlackHoleConfig::default()
	}))
	.set_all_levels();

	r::set!(log, a_string = "hello there!", an_int = 12345, a_float = 6789.0123 as f32);

	log
}

fn run(bencher: Bencher, mut log: Logger) {
	bencher
		.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
		.with_inputs(|| BENCHMARK_LOG_ITEMS)
		.bench_local_refs(|total| {
			for i in 0..*total {
				r::info!(log, "benchmark test!", iteration = i);
			}
		});
}

#[divan::bench]
fn none(bencher: Bencher) {
	let log = init_logger();
	run(bencher, log);
}

mod level {
	use super::*;

	#[divan::bench]
	fn levels(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::levels::Levels::new(filter::levels::LevelsConfig {
			levels: [Level::Debug, Level::Info, Level::Fatal, Level::Panic],
		}));
		run(bencher, log);
	}
}

mod matches {
	use super::*;

	#[divan::bench]
	fn message(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::matches::Message::new(filter::matches::MessageConfig {
			has: ["test!"],
			has_not: ["bRoKeN", "mSg"],
			match_all: true,
		}));
		run(bencher, log);
	}

	#[divan::bench]
	fn attr_key(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::matches::AttributeKey::new(filter::matches::AttributeKeyConfig {
			has: ["an_int", "a_string"],
			has_not: ["a_bool"],
			match_all: true,
		}));
		run(bencher, log);
	}

	#[divan::bench]
	fn attr_value(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::matches::AttributeValue::new(filter::matches::AttributeValueConfig {
			key: "a_string",
			has: ["hello", "there"],
			has_not: ["12345", "6789"],
			match_all: true,
		}));
		run(bencher, log);
	}
}

mod sample {
	use super::*;
	use ntime::Duration;

	#[divan::bench]
	fn random(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::sample::Random::new(filter::sample::RandomConfig { probability: 0.625 }));
		run(bencher, log);
	}

	#[divan::bench]
	fn step(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::sample::Step::new(filter::sample::StepConfig { step: 170 }));
		run(bencher, log);
	}

	#[divan::bench]
	fn random_step(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::sample::RandomStep::new(filter::sample::RandomStepConfig { step: 7 }));
		run(bencher, log);
	}

	#[divan::bench]
	fn burst(bencher: Bencher) {
		let mut log = init_logger();
		log.add_filter(filter::sample::Burst::new(filter::sample::BurstConfig {
			period: Duration::from_millis(100),
			max_messages: 1500,
		}));
		run(bencher, log);
	}
}
