// TODO: switch over to 'cargo bench`, once that feature finally becomes stable >:(

// benchmarks should not be executed in parallel, so we declare them here, and
// launch them in sequence below.
#[cfg(all(test, feature = "benchmark"))]
mod benchmark {
	use super::*;

	use rasant::attributes::value::ToValue;
	use rasant::level::Level;
	use rasant::sink::black_hole;
	use rasant::sink::format::{FormatterConfig, OutputFormat};
	use rasant::time::{Duration, StringFormat, Timestamp};
	use std::thread;

	const BENCHMARK_LOG_ITEMS: u32 = 1000000;
	const BENCHMARK_MAX_NESTING: u32 = 50;

	pub fn black_hole_single(async_writes: bool, skip_all: bool, log_format: OutputFormat) -> (u32, Duration) {
		let mut log = rasant::Logger::new();
		log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
			formatter_cfg: FormatterConfig {
				format: log_format,
				time_format: StringFormat::TimestampNanoseconds,
				..FormatterConfig::default()
			},
			..black_hole::BlackHoleConfig::default()
		}));
		log.set_async(async_writes);
		log.set_level(if skip_all { Level::Warning } else { Level::Info });

		let total = BENCHMARK_LOG_ITEMS;
		let start = Timestamp::now();
		{
			for i in 0..total {
				log.info_with("benchmark test!", [("iteration", i.to_value())]);
			}
		}
		let runtime = Timestamp::now() - start;

		(total, runtime)
	}

	pub fn black_hole_nested(async_writes: bool, skip_all: bool, log_format: OutputFormat) -> (u32, Duration) {
		let mut log = rasant::Logger::new();
		log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
			formatter_cfg: FormatterConfig {
				format: log_format,
				time_format: StringFormat::TimestampNanoseconds,
				..FormatterConfig::default()
			},
			..black_hole::BlackHoleConfig::default()
		}));
		log.set_async(async_writes);
		log.set_level(if skip_all { Level::Warning } else { Level::Info });

		let total = BENCHMARK_LOG_ITEMS;
		let entries_per_logger = BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_NESTING;
		let start = Timestamp::now();
		let mut logger_count = 0;
		{
			for i in 0..total {
				if i % entries_per_logger == 0 {
					logger_count += 1;
					log = log.clone();
					log.set("logger", logger_count);
				}
				log.info_with("benchmark test!", [("iteration", i.to_value())]);
			}
		}
		let runtime = Timestamp::now() - start;

		(total, runtime)
	}

	pub fn black_hole_nested_with_arguments(async_writes: bool, skip_all: bool, log_format: OutputFormat) -> (u32, Duration) {
		let mut log = rasant::Logger::new();
		log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
			formatter_cfg: FormatterConfig {
				format: log_format,
				time_format: StringFormat::TimestampNanoseconds,
				..FormatterConfig::default()
			},
			..black_hole::BlackHoleConfig::default()
		}));
		log.set_async(async_writes);
		log.set_level(if skip_all { Level::Warning } else { Level::Info });

		let total = BENCHMARK_LOG_ITEMS;
		let entries_per_logger = BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_NESTING;
		let mut logger_count = 0;
		let start = Timestamp::now();
		{
			for i in 0..total {
				if i % entries_per_logger == 0 {
					logger_count += 1;
					log = log.clone();

					let test_key = format!("key_{logger_count}");
					log.set(test_key.as_str(), logger_count);
				}
				log.info_with("benchmark test!", [("iteration", i.to_value())]);
			}
		}
		let runtime = Timestamp::now() - start;

		(total, runtime)
	}

	pub fn black_hole_threaded(async_writes: bool, skip_all: bool, log_format: OutputFormat) -> (u32, Duration) {
		let mut log = rasant::Logger::new();
		log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
			formatter_cfg: FormatterConfig {
				format: log_format,
				time_format: StringFormat::TimestampNanoseconds,
				..FormatterConfig::default()
			},
			..black_hole::BlackHoleConfig::default()
		}));
		log.set_async(async_writes);
		log.set_level(if skip_all { Level::Warning } else { Level::Info });

		let total = BENCHMARK_LOG_ITEMS;
		let thread_count = BENCHMARK_MAX_NESTING;
		let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
		let start = Timestamp::now();
		{
			for i in 0..thread_count {
				let mut tlog = log.clone();
				tlog.set("thread_num", i);
				let msgs_per_thread = total / thread_count;
				handles.push(thread::spawn(move || {
					for j in 0..msgs_per_thread {
						tlog.info_with("threaded benchmark test", [("iteration", j.to_value())]);
					}
				}));
			}

			for h in handles {
				h.join().expect("failed to close benchmark logging thread");
			}
		}
		let runtime = Timestamp::now() - start;

		(total, runtime)
	}

	#[test]
	fn run() {
		struct Benchmark {
			name: String,
			func: fn(async_writes: bool, skip_all: bool, log_format: OutputFormat) -> (u32, Duration),
		}

		let benchmarks: [Benchmark; _] = [
			Benchmark {
				name: "single logger".into(),
				func: black_hole_single,
			},
			Benchmark {
				name: format!("{} nested loggers", BENCHMARK_MAX_NESTING),
				func: black_hole_nested,
			},
			Benchmark {
				name: format!("{} nested loggers with increasing arguments", BENCHMARK_MAX_NESTING),
				func: black_hole_nested_with_arguments,
			},
			Benchmark {
				name: format!("{} multi-threaded nested loggers", BENCHMARK_MAX_NESTING),
				func: black_hole_threaded,
			},
		];

		for b in benchmarks {
			println!("--- Benchmark: {name} ---", name = b.name);
			for async_writes in [false, true] {
				println!("[{}]", if async_writes { "async" } else { "sync" });

				for skip_all in [false, true] {
					for log_format in [OutputFormat::Compact, OutputFormat::Json] {
						let (total, runtime) = (b.func)(async_writes, skip_all, log_format.clone());
						println!(
							"\t{op} {total} {format} log entries in {runtime:?}, average {avg:?}/op",
							op = if skip_all { "skipped" } else { "wrote" },
							avg = runtime / total,
							format = log_format.name(),
						);
					}
				}
			}
			println!("");
		}
	}
}
