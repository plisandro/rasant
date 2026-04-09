use ntime;
use rasant::Level;
use rasant::sink;
use rasant::{FormatterConfig, OutputFormat};

use std::io::{Error, ErrorKind};

#[test]
fn sync_output() {
	struct TestCase<'t> {
		name: &'t str,
		out_format: OutputFormat,
		time_format: ntime::Format,
		want: &'t str,
	}

	let test_cases: [TestCase; _] = [
		TestCase {
			name: "default stdout",
			out_format: OutputFormat::Compact,
			time_format: ntime::Format::UtcMillisDateTime,
			want: "2026-03-04 15:10:15.000 [INF] root test info
2026-03-04 15:10:16.234 [WRN] root test warn
2026-03-04 15:10:17.468 [INF] first test info number=1
2026-03-04 15:10:18.702 [WRN] first test warn number=1
2026-03-04 15:10:19.936 [ERR] something failed error=\"oh no\" number=1",
		},
		TestCase {
			name: "stdout with timestamps",
			out_format: OutputFormat::Compact,
			time_format: ntime::Format::TimestampNanoseconds,
			want: "1772637015000000000 [INF] root test info
1772637016234000000 [WRN] root test warn
1772637017468000000 [INF] first test info number=1
1772637018702000000 [WRN] first test warn number=1
1772637019936000000 [ERR] something failed error=\"oh no\" number=1",
		},
		TestCase {
			name: "JSON stdout",
			out_format: OutputFormat::Json,
			time_format: ntime::Format::UtcDateTime,
			want: "{\"time\":\"2026-03-04 15:10:15\",\"level\":\"info\",\"message\":\"root test info\"}
{\"time\":\"2026-03-04 15:10:16\",\"level\":\"warning\",\"message\":\"root test warn\"}
{\"time\":\"2026-03-04 15:10:17\",\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"time\":\"2026-03-04 15:10:18\",\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"time\":\"2026-03-04 15:10:19\",\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
		},
		TestCase {
			name: "JSON stdout with timestamps",
			out_format: OutputFormat::Json,
			time_format: ntime::Format::TimestampMilliseconds,
			want: "{\"timestamp\":1772637015000,\"level\":\"info\",\"message\":\"root test info\"}
{\"timestamp\":1772637016234,\"level\":\"warning\",\"message\":\"root test warn\"}
{\"timestamp\":1772637017468,\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"timestamp\":1772637018702,\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"timestamp\":1772637019936,\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
		},
	];

	for tc in test_cases {
		let string_sink = sink::string::String::new(sink::string::StringConfig {
			mock_time: true,
			formatter_cfg: FormatterConfig {
				format: tc.out_format,
				time_format: tc.time_format,
				..FormatterConfig::default()
			},
			..sink::string::StringConfig::default()
		});
		let string_sink_output = string_sink.output();

		{
			let mut log = rasant::Logger::new();
			log.add_sink(string_sink).set_level(Level::Info);

			log.info("root test info").warn("root test warn").debug("root test debug");

			let mut nlog = log.clone();
			nlog.set("number", 1);
			nlog.info("first test info")
				.warn("first test warn")
				.debug("first test debug, ignore me")
				.error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");
		}

		let got = string_sink_output.lock().unwrap().clone();
		assert_eq!(got, tc.want, "{}", tc.name);
	}
}

#[test]
fn sync_trace() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Trace).add_sink(string_sink);

		log.info("root test info").warn("root test warn").debug("root test debug");

		let mut nlog = log.clone();
		nlog.set("number", 1);
		nlog.info("first test info")
			.warn("first test warn")
			.debug("first test debug")
			.error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [TRA] added new log sink name=\"default log string\" async=false logger_id=100
2026-03-04 15:10:16.234 [INF] root test info
2026-03-04 15:10:17.468 [WRN] root test warn
2026-03-04 15:10:18.702 [DBG] root test debug
2026-03-04 15:10:19.936 [INF] first test info number=1
2026-03-04 15:10:21.170 [WRN] first test warn number=1
2026-03-04 15:10:22.404 [DBG] first test debug number=1
2026-03-04 15:10:23.638 [ERR] something failed error=\"oh no\" number=1";

	assert_eq!(got, want);
}
