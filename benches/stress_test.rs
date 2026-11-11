use std::sync::{Arc, Barrier};
use std::thread;

use divan::{Bencher, counter};
use rasant as r;

const LOGGER_WRITES_PER_THREAD: usize = 500;
const LOGGER_THREADS: &[usize] = &[1, 10, 50, 100, 250, 1000, 2500];

fn main() {
	divan::main();
}

fn init_logger() -> r::Logger {
	let mut log = r::Logger::new();
	log.add_sink(r::sink::black_hole::BlackHole::new(r::sink::black_hole::BlackHoleConfig {
		formatter_cfg: r::FormatterConfig {
			format: r::OutputFormat::Compact,
			time_format: r::TimeFormat::TimestampNanoseconds,
			..r::FormatterConfig::default()
		},
		..r::sink::black_hole::BlackHoleConfig::default()
	}))
	.set_all_levels()
	.set_async(true);

	log
}

// busy-waits for Rasant's global async handler to drain and shut down
fn wait_for_async_drain() {
	loop {
		let status = r::get_status();
		if status.async_queue_size == 0 {
			break;
		}
		thread::yield_now();
	}
}

mod async_handler {
	use super::*;

	// benchmarks global async queue performance upon high write contention.
	#[divan::bench(consts=LOGGER_THREADS)]
	fn high_contention<const N: usize>(bencher: Bencher) {
		bencher
			.counter(counter::ItemsCount::new(N * LOGGER_WRITES_PER_THREAD))
			.with_inputs(|| (init_logger(), N, LOGGER_WRITES_PER_THREAD))
			.bench_local_values(|(log, thread_count, writes_per_thread)| {
				let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
				// ensure writer threads all start together
				let barrier = Arc::new(Barrier::new(N));

				for _ in 0..thread_count {
					let mut log = log.clone();
					let barrier = barrier.clone();

					handles.push(thread::spawn(move || {
						barrier.wait();
						for i in 0..writes_per_thread {
							// perform somewhat realsitic log writes
							match i % 4 {
								0 => r::trace!(log, "trace test", key = 123456),
								1 => r::debug!(log, "debug test", short_string = "this is a static string"),
								2 => r::info!(
									log,
									"info test",
									heap_string = String::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.")
								),
								_ => r::warn!(log, "warn test", a_map = r::map!("a" => "lalala", "b" => "lololo", "c" => 123 as u8, "d" => 456 as usize)),
							};
						}
					}));
				}

				// wait for all writes to complete...
				for h in handles {
					h.join().unwrap();
				}

				// ...and drain.
				wait_for_async_drain();
			});
	}

	// benchmarks lock time (not end-to-end writes) for log operations on async loggers.
	#[divan::bench(threads=LOGGER_THREADS)]
	fn lock_time(bencher: Bencher) {
		let log = init_logger();

		bencher
			.counter(counter::ItemsCount::new(LOGGER_WRITES_PER_THREAD))
			.with_inputs(|| (log.clone(), LOGGER_WRITES_PER_THREAD))
			.bench_local_values(|(mut log, writes_per_thread)| {
				for _ in 0..writes_per_thread {
					// log writes are minimal to ensure we measure lock time and not f.ex. attributes intialization.
					log.info("cheap log op");
				}
			});

		wait_for_async_drain();
	}
}
