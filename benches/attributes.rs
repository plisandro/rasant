use divan::Bencher;
use rasant as r;
use rasant::sink::black_hole;
use rasant::{FormatterConfig, Logger, OutputFormat, TimeFormat, ToValue, Value};

const COUNTS: &[usize] = &[0, 1, 5, 10, 25, 50, 100, 250];

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

fn build_attrs(total: usize) -> Vec<(String, Value)> {
	let mut res: Vec<(String, Value)> = Vec::new();

	for i in 0..total {
		res.push((
			format!("key_{i}"),
			match i % 4 {
				0 => true.to_value(),
				1 => "lalala".to_value(),
				2 => 123.to_value(),
				_ => (456.789 as f32).to_value(),
			},
		));
	}

	res
}

// Benchmarks attributes defined only for the logger.
#[divan::bench(consts=COUNTS)]
fn in_logger<const N: usize>(bencher: Bencher) {
	let mut log = init_logger();

	for attr in build_attrs(N) {
		log.set_value(attr.0.as_str(), attr.1);
	}

	bencher.bench_local(|| {
		r::info!(log, "attributes benchmark test");
	});
}

// Benchmarks attributes defined only for the log update.
#[divan::bench(consts=COUNTS)]
fn in_update<const N: usize>(bencher: Bencher) {
	let log = init_logger();
	let attrs = build_attrs(N);
	let attrs = attrs.iter().map(|x| (x.0.as_str(), x.1.clone())).collect::<Vec<(&str, Value)>>();
	let attrs = attrs.as_array::<N>().unwrap();

	bencher.with_inputs(|| (log.clone(), attrs.clone())).bench_values(|(mut log, attrs)| {
		log.info_with::<N>("attributes benchmark test", attrs);
	});
}
