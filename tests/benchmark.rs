use std::thread;

use slog;
use slog::attributes::value::ToValue;
use slog::level::Level;
use slog::sink;
use slog::time::Timestamp;

const BENCHMARK_LOG_ITEMS: u32 = 1000000;
const BENCHMARK_MAX_NESTING: u32 = 50;

#[cfg(test)]
mod benchmark_tests {
	use super::*;

	#[test]
	fn black_hole_single() {
		let mut log = slog::Slog::new();
		log.add_sink(sink::black_hole::default()).set_level(Level::Info);

		let total = BENCHMARK_LOG_ITEMS;
		let start = Timestamp::now();
		{
			for i in 0..total {
				log.info_with("benchmark test!", [("iteration", i.to_value())]);
			}
		}
		let runtime = Timestamp::now() - start;

		println!("wrote {total} log entries in {runtime:?}, average {avg:?}/op", avg = runtime / total);
	}

	#[test]
	fn black_hole_nested() {
		let mut log = slog::Slog::new();
		log.add_sink(sink::black_hole::default()).set_level(Level::Info);

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

		println!(
			"wrote {total} log entries in {runtime:?} via {logger_count} logger instances, average {avg:?}/op",
			avg = runtime / total
		);
	}

	#[test]
	fn black_hole_nested_with_arguments() {
		let mut log = slog::Slog::new();
		log.add_sink(sink::black_hole::default()).set_level(Level::Info);

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

		println!(
			"wrote {total} log entries in {runtime:?} via {logger_count} logger instances with up to {logger_count} arguments, average {avg:?}/op",
			avg = runtime / total
		);
	}

	/*
	#[test]
	fn black_hole_threaded() {
		let mut log = slog::Slog::new();
		log.add_sink(sink::black_hole::default()).set_level(Level::Info);

		let total = BENCHMARK_LOG_ITEMS;
		let thread_count = BENCHMARK_MAX_NESTING;
		let handles: Vec<thread::JoinHandle<()>> = Vec::new();
		let start = Timestamp::now();
		{
			for i in 0..thread_count {
				let tlog = log.clone().set("thread_num", i);
				let msgs_per_thread = total / thread_count;
				handles.push(thread::spawn(move || {
					for j in 0..msgs_per_thread {
						tlog.info_with("threaded benchmark test", [("iteration", j.to_value())]);
					}
				}));
			}

			for h in handles {
				h.join();
			}
		}
		let runtime = Timestamp::now() - start;

		println!(
			"wrote {total} log entries in {runtime:?} threaded {thread_count} thread logger instances, average {avg:?}/op",
			avg = runtime / total
		);
	}
	*/
}
