use divan::{Bencher, counter};
use rasant as r;
use rasant::sink::black_hole::{BlackHole, BlackHoleConfig};
use rasant::sink::syslog::{Syslog, SyslogConfig, SyslogFormat};
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

#[cfg(unix)]
#[divan::bench]
fn journald(bencher: Bencher) {
	use rasant::sink::journald;

	let sink = journald::black_hole();

	run(bencher, sink);
}

#[divan::bench]
fn syslog_3164(bencher: Bencher) {
	let sink = Syslog::new(SyslogConfig {
		format: SyslogFormat::RFC3164,
		..SyslogConfig::default_black_hole()
	});

	run(bencher, sink);
}

#[divan::bench]
fn syslog_5424(bencher: Bencher) {
	let sink = Syslog::new(SyslogConfig {
		format: SyslogFormat::RFC5424,
		..SyslogConfig::default_black_hole()
	});

	run(bencher, sink);
}

#[divan::bench]
fn syslog_5424_full(bencher: Bencher) {
	let sink = Syslog::new(SyslogConfig {
		format: SyslogFormat::RFC5424Full,
		..SyslogConfig::default_black_hole()
	});

	run(bencher, sink);
}
