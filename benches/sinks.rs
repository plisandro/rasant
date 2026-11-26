use divan::{Bencher, counter};
use rasant as r;
use rasant::sink::black_hole::{BlackHole, BlackHoleConfig};
use rasant::sink::syslog::{Syslog, SyslogConfig, SyslogFormat};
use rasant::{FormatterConfig, OutputFormat, TimeFormat, TimePrecision};

const BENCHMARK_LOG_ITEMS: usize = 10000;
const IO_FORMATS: &[OutputFormat] = &[
	OutputFormat::Compact,
	OutputFormat::ColorCompact,
	OutputFormat::Full,
	OutputFormat::ColorFull,
	OutputFormat::Json,
	OutputFormat::Cbor,
];
const SYSLOG_FORMATS: &[SyslogFormat] = &[SyslogFormat::Rfc3164, SyslogFormat::Rfc5424, SyslogFormat::Rfc5424Full];

fn main() {
	divan::main()
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

mod io {
	use super::*;

	fn bh_sink(format: OutputFormat, in_memory: bool) -> BlackHole {
		BlackHole::new(BlackHoleConfig {
			formatter_cfg: FormatterConfig {
				format,
				time_format: TimeFormat::Timestamp(TimePrecision::Nanoseconds),
				..FormatterConfig::default()
			},
			in_memory,
		})
	}

	#[divan::bench(args=IO_FORMATS)]
	fn io_empty(bench: Bencher, format: &OutputFormat) {
		let sink = bh_sink(format.clone(), false);
		run(bench, sink);
	}

	#[divan::bench(args=IO_FORMATS)]
	fn mem_buffer(bench: Bencher, format: &OutputFormat) {
		let sink = bh_sink(format.clone(), true);
		run(bench, sink);
	}
}

#[divan::bench(args=SYSLOG_FORMATS)]
fn syslog(bencher: Bencher, format: &SyslogFormat) {
	let sink = Syslog::new(SyslogConfig {
		format: format.clone(),
		..SyslogConfig::default_black_hole()
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
