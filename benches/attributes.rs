use divan::{Bencher, counter};
use rasant as r;
use rasant::sink::black_hole;
use rasant::{FormatterConfig, Logger, OutputFormat, Scalar, TimeFormat, Value};

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

fn build_attrs<'i>(total: usize) -> Vec<(String, Value<'i>)> {
	let mut res: Vec<(String, Value)> = Vec::new();

	for i in 0..total {
		res.push((
			format!("key_{i}"),
			match i % 4 {
				0 => Value::from(true),
				1 => Value::from("lalala"),
				2 => Value::from(123),
				_ => Value::from(456.789),
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
		log.set(attr.0.as_str(), attr.1);
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

	bencher.with_inputs(|| (log.clone(), attrs.clone())).bench_local_values(|(mut log, attrs)| {
		log.info_with::<N>("attributes benchmark test", attrs);
	});
}

// Benchmark performance of attribute overwrites in maps.
#[divan::bench(consts=COUNTS)]
fn key_overwrite<const N: usize>(bencher: Bencher) {
	let key = "test";
	let short_string = "this is a static string";
	let long_string = String::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");

	let mut log = init_logger();
	log.set(key, "");

	bencher.counter(counter::ItemsCount::new(N)).with_inputs(|| log.clone()).bench_local_values(|mut log| {
		for i in 0..N {
			match i % 5 {
				0 => log.set(key, 123456),
				1 => log.set(key, short_string),
				2 => log.set(key, long_string.clone()),
				3 => log.set(key, &[Scalar::from(123.456), Scalar::from(short_string), Scalar::from(long_string.clone())]),
				_ => log.set(
					key,
					(
						&[Scalar::from("key_a"), Scalar::from("key_b"), Scalar::from("key_c")],
						&[Scalar::from(123.456), Scalar::from(short_string), Scalar::from(long_string.clone())],
					),
				),
			};
		}
	});
}
