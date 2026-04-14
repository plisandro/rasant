use divan::{Bencher, counter};
use rasant as r;
use rasant::sink::black_hole;
use rasant::{FormatterConfig, OutputFormat, TimeFormat};

const BENCHMARK_LOG_ITEMS: usize = 10000;

fn main() {
	divan::main();
}

fn run(bench: Bencher, output_format: OutputFormat) {
	let mut log = rasant::Logger::new();
	log.add_sink(black_hole::BlackHole::new(black_hole::BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: output_format,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..black_hole::BlackHoleConfig::default()
	}))
	.set_all_levels();

	bench
		.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
		.with_inputs(|| BENCHMARK_LOG_ITEMS)
		.bench_local_refs(|total| {
			for i in 0..*total {
				r::info!(log, "benchmark test!", iteration = i);
			}
		});
}

#[divan::bench]
fn compact(bench: Bencher) {
	run(bench, OutputFormat::Compact);
}

#[divan::bench]
fn json(bench: Bencher) {
	run(bench, OutputFormat::Json);
}
