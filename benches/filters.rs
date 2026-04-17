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

#[divan::bench]
fn levels(bencher: Bencher) {
	let mut log = init_logger();
	log.add_filter(filter::levels::Levels::new(filter::levels::LevelsConfig {
		levels: [Level::Debug, Level::Info, Level::Fatal, Level::Panic],
	}));
	run(bencher, log);
}
