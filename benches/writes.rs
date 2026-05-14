use divan::{Bencher, counter};
use rasant as r;
use rasant::sink;
use rasant::sink::black_hole::{BlackHole, BlackHoleConfig};
use rasant::{FormatterConfig, OutputFormat, TimeFormat};

const BENCHMARK_LOG_ITEMS: usize = 10000;

fn main() {
	divan::main();
}

fn run<T: rasant::sink::Sink + 'static + Send>(bencher: Bencher, sink: T) {
	let mut log = rasant::Logger::new();
	log.add_sink(sink).set_all_levels();

	bencher
		.counter(counter::ItemsCount::new(BENCHMARK_LOG_ITEMS))
		.with_inputs(|| BENCHMARK_LOG_ITEMS)
		.bench_local_refs(move |total| {
			for i in 0..*total {
				r::info!(log, "benchmark test!", iteration = i);
			}
		});
}

#[divan::bench]
fn io_compact(bencher: Bencher) {
	let sink = BlackHole::new(BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::Compact,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..BlackHoleConfig::default()
	});

	run(bencher, sink);
}

#[divan::bench]
fn io_color_compact(bencher: Bencher) {
	let sink = BlackHole::new(BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::ColorCompact,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..BlackHoleConfig::default()
	});

	run(bencher, sink);
}

#[divan::bench]
fn io_json(bencher: Bencher) {
	let sink = BlackHole::new(BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::Json,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..BlackHoleConfig::default()
	});

	run(bencher, sink);
}

#[divan::bench]
fn io_cbor(bencher: Bencher) {
	let sink = BlackHole::new(BlackHoleConfig {
		formatter_cfg: FormatterConfig {
			format: OutputFormat::Cbor,
			time_format: TimeFormat::TimestampNanoseconds,
			..FormatterConfig::default()
		},
		..BlackHoleConfig::default()
	});

	run(bencher, sink);
}

#[divan::bench]
fn journald(bencher: Bencher) {
	let sink = sink::journald::black_hole();

	run(bencher, sink);
}
