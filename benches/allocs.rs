use divan::{AllocProfiler, Bencher, counter};
use rasant as r;
use rasant::Logger;
use rasant::sink::black_hole;

const BENCHMARK_LOG_ITEMS: usize = 10000;
const BENCHMARK_MAX_ARGUMENTS: usize = 100;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
	divan::main();
}

fn init_logger() -> Logger {
	let mut log = rasant::Logger::new();
	log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
		formatter_cfg: r::FormatterConfig::default_compact(),
		..black_hole::BlackHoleConfig::default()
	}))
	.set_all_levels();

	log
}

mod no_arguments {
	use super::*;

	fn run(bencher: Bencher, mut log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| BENCHMARK_LOG_ITEMS)
			.bench_local_refs(|total| {
				for _ in 0..*total {
					r::info!(log, "benchmark test!");
				}
			});
	}

	#[divan::bench(name = "async")]
	fn async_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench(name = "sync")]
	fn sync_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(false);
		run(bencher, log);
	}
}

mod multi_argument {
	use super::*;

	fn run(bencher: Bencher, mut log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| BENCHMARK_LOG_ITEMS)
			.bench_local_refs(|total| {
				for i in 0..*total {
					if i % (BENCHMARK_LOG_ITEMS / BENCHMARK_MAX_ARGUMENTS) == 0 {
						log.set(format!("arg_{}", i).as_str(), 123456);
					}
					r::info!(log, "benchmark test!");
				}
			});
	}

	#[divan::bench(name = "async")]
	fn async_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench(name = "sync")]
	fn sync_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(false);
		run(bencher, log);
	}
}
mod single_argument {
	use super::*;

	fn run(bencher: Bencher, mut log: Logger) {
		bencher
			.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
			.with_inputs(|| BENCHMARK_LOG_ITEMS)
			.bench_local_refs(|total| {
				r::set!(log, some_bool = true, short_string = "hello there!", a_float = 3.1415926, usize = 34834939 as usize);
				r::set!(
					log,
					long_string = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
				);
				for i in 0..*total {
					r::info!(log, "benchmark test!", iteration = i);
				}
			});
	}

	#[divan::bench(name = "async")]
	fn async_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(true);
		run(bencher, log);
	}

	#[divan::bench(name = "sync")]
	fn sync_mode(bencher: Bencher) {
		let mut log = init_logger();
		log.set_async(false);
		run(bencher, log);
	}
}
