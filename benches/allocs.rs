use divan::{AllocProfiler, Bencher};
use rasant as r;
use rasant::Logger;
use rasant::sink::black_hole;

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
		bencher.bench_local(|| {
			r::info!(log, "benchmark test!");
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
		bencher.bench_local(move || {
			r::info!(log, "benchmark test!", foo = 12345);
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
		bencher.bench_local(move || {
			log.set("some_bool", true);
			log.set("short_string", "hello_there");
			log.set("a_float", 3.1415926);
			log.set("an_usize", 374943849439 as usize);
			r::info!(log, "benchmark test!", foo = 12345);
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

mod with_long_strings {
	use super::*;

	fn run(bencher: Bencher, mut log: Logger) {
		bencher.bench_local(move || {
			log.set("some_bool", true);
			log.set("short_string", "hello_there");
			log.set(
				"long_string",
				"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
			);
			log.set("a_float", 3.1415926);
			log.set("an_usize", 374943849439 as usize);
			r::info!(log, "benchmark test!", foo = 12345);
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
