use std::fmt;

use divan::{Bencher, counter};
use rasant as r;
use rasant::sink::black_hole;
use rasant::{FormatterConfig, Level, Logger, OutputFormat, TimeFormat, TimePrecision};

const BENCHMARK_LOG_ITEMS: usize = 10000;
const BENCHMARK_MAX_NESTING: usize = 50;
const BLACKHOLE_MODES: &[BlackHoleMode] = &[BlackHoleMode::IoEmpty, BlackHoleMode::InMemory];

#[derive(Copy, Clone)]
enum BlackHoleMode {
	IoEmpty,
	InMemory,
}

impl fmt::Display for BlackHoleMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			BlackHoleMode::IoEmpty => write!(f, "io_empty"),
			BlackHoleMode::InMemory => write!(f, "mem_buffer"),
		}
	}
}

fn main() {
	divan::main();
}

fn init_logger(mode: &BlackHoleMode) -> Logger {
	let mut log = rasant::Logger::new();
	log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::Compact,
			time_format: TimeFormat::Timestamp(TimePrecision::Nanoseconds),
			..FormatterConfig::default()
		},
		in_memory: matches!(mode, BlackHoleMode::InMemory),
	}))
	.set_all_levels();

	log
}

mod single {
	use super::*;

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

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn write(bencher: Bencher, mode: &BlackHoleMode) {
		let log = init_logger(mode);
		run(bencher, log);
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn async_write(bencher: Bencher, mode: &BlackHoleMode) {
		let mut log = init_logger(mode);
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench]
	fn skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_level(Level::Warning);
		run(bencher, log);
	}

	#[divan::bench]
	fn async_skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_async(true).set_level(Level::Warning);
		run(bencher, log);
	}
}

mod nested {
	use super::*;

	fn run(bencher: Bencher, log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| (BENCHMARK_LOG_ITEMS, BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_NESTING))
			.bench_local_refs(|(total, entries_per_logger)| {
				let mut log = log.clone();
				let mut logger_count = 0;
				for i in 0..*total {
					if i % *entries_per_logger == 0 {
						logger_count += 1;
						log = log.clone();
						log.set("logger", logger_count);
					}
					r::info!(log, "benchmark test!", iteration = i);
				}
			});
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn write(bencher: Bencher, mode: &BlackHoleMode) {
		let log = init_logger(mode);
		run(bencher, log);
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn async_write(bencher: Bencher, mode: &BlackHoleMode) {
		let mut log = init_logger(mode);
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench]
	fn skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_level(Level::Warning);
		run(bencher, log);
	}

	#[divan::bench]
	fn async_skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_async(true).set_level(Level::Warning);
		run(bencher, log);
	}
}

mod nested_with_arguments {
	use super::*;

	fn run(bencher: Bencher, log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| (BENCHMARK_LOG_ITEMS, BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_NESTING))
			.bench_local_refs(|(total, entries_per_logger)| {
				let mut log = log.clone();
				let mut logger_count = 0;
				for i in 0..*total {
					if i % *entries_per_logger == 0 {
						logger_count += 1;
						log = log.clone();
						let test_key = format!("key_{logger_count}");
						log.set(test_key.as_str(), logger_count);
					}
					r::info!(log, "benchmark test!", iteration = i);
				}
			});
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn write(bencher: Bencher, mode: &BlackHoleMode) {
		let log = init_logger(mode);
		run(bencher, log);
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn async_write(bencher: Bencher, mode: &BlackHoleMode) {
		let mut log = init_logger(mode);
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench]
	fn skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_level(Level::Warning);
		run(bencher, log);
	}

	#[divan::bench]
	fn async_skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_async(true).set_level(Level::Warning);
		run(bencher, log);
	}
}

mod threaded {
	use super::*;
	use std::thread;

	fn run(bencher: Bencher, log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| (BENCHMARK_MAX_NESTING, BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_NESTING))
			.bench_local_refs(|(thread_count, entries_per_logger)| {
				let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
				for i in 0..*thread_count {
					let mut tlog = log.clone();
					tlog.set("thread_num", i);
					let entries = *entries_per_logger;
					handles.push(thread::spawn(move || {
						for j in 0..entries {
							r::info!(tlog, "threaded benchmark test", iteration = j);
						}
					}));
				}

				for h in handles {
					h.join().expect("failed to close benchmark logging thread");
				}
			});
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn write(bencher: Bencher, mode: &BlackHoleMode) {
		let log = init_logger(mode);
		run(bencher, log);
	}

	#[divan::bench(args=BLACKHOLE_MODES)]
	fn async_write(bencher: Bencher, mode: &BlackHoleMode) {
		let mut log = init_logger(mode);
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench]
	fn skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_level(Level::Warning);
		run(bencher, log);
	}

	#[divan::bench]
	fn async_skip(bencher: Bencher) {
		let mut log = init_logger(&BlackHoleMode::IoEmpty);
		log.set_async(true).set_level(Level::Warning);
		run(bencher, log);
	}
}
