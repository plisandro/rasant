use slog;
use slog::level::Level;
use slog::sink;
use slog::time;

use std::io::{Error, ErrorKind};
use std::sync::Mutex;

#[test]
fn formatted_output() {
	struct TestCase<'t> {
		name: &'t str,
		out_format: sink::format::OutputFormat,
		time_format: time::StringFormat,
		want: &'t str,
	}

	let test_cases: [TestCase; _] = [
		TestCase {
			name: "default stdout",
			out_format: sink::format::OutputFormat::Compact,
			time_format: time::format::StringFormat::UtcMillisDateTime,
			want: "2026-03-04 15:10:15.000 [INF] log level updated name=\"info\" new_level=2
2026-03-04 15:10:16.234 [INF] root test info
2026-03-04 15:10:17.468 [WRN] root test warn
2026-03-04 15:10:18.702 [INF] first test info number=1
2026-03-04 15:10:19.936 [WRN] first test warn number=1
2026-03-04 15:10:21.170 [ERR] something failed error=\"oh no\" number=1",
		},
		TestCase {
			name: "stdout with timestamps",
			out_format: sink::format::OutputFormat::Compact,
			time_format: time::format::StringFormat::TimestampNanoseconds,
			want: "1772637015000000000 [INF] log level updated name=\"info\" new_level=2
1772637016234000000 [INF] root test info
1772637017468000000 [WRN] root test warn
1772637018702000000 [INF] first test info number=1
1772637019936000000 [WRN] first test warn number=1
1772637021170000000 [ERR] something failed error=\"oh no\" number=1",
		},
		TestCase {
			name: "JSON stdout",
			out_format: sink::format::OutputFormat::Json,
			time_format: time::format::StringFormat::UtcDateTime,
			want: "{\"time\":\"2026-03-04 15:10:15\",\"level\":\"info\",\"message\":\"log level updated\",\"name\":\"info\",\"new_level\":2}
{\"time\":\"2026-03-04 15:10:16\",\"level\":\"info\",\"message\":\"root test info\"}
{\"time\":\"2026-03-04 15:10:17\",\"level\":\"warning\",\"message\":\"root test warn\"}
{\"time\":\"2026-03-04 15:10:18\",\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"time\":\"2026-03-04 15:10:19\",\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"time\":\"2026-03-04 15:10:21\",\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
		},
		TestCase {
			name: "JSON stdout with timestamps",
			out_format: sink::format::OutputFormat::Json,
			time_format: time::format::StringFormat::TimestampMilliseconds,
			want: "{\"timestamp\":1772637015000,\"level\":\"info\",\"message\":\"log level updated\",\"name\":\"info\",\"new_level\":2}
{\"timestamp\":1772637016234,\"level\":\"info\",\"message\":\"root test info\"}
{\"timestamp\":1772637017468,\"level\":\"warning\",\"message\":\"root test warn\"}
{\"timestamp\":1772637018702,\"level\":\"info\",\"message\":\"first test info\",\"number\":1}
{\"timestamp\":1772637019936,\"level\":\"warning\",\"message\":\"first test warn\",\"number\":1}
{\"timestamp\":1772637021170,\"level\":\"error\",\"message\":\"something failed\",\"error\":\"oh no\",\"number\":1}",
		},
	];

	for tc in test_cases {
		let mut log_out = Mutex::new(String::from(""));

		{
			let mut log = slog::Slog::new();
			log.add_sink(sink::string::String::new(sink::string::StringConfig {
				out: Some(&log_out),
				mock_time: true,
				formatter_cfg: sink::format::FormatterConfig {
					format: tc.out_format,
					time_format: tc.time_format,
				},
				..sink::string::StringConfig::default()
			}))
			.set_level(Level::Info);

			log.info("root test info").warn("root test warn").debug("root test debug");

			let mut nlog = log.clone();
			nlog.set("number", 1);
			nlog.info("first test info")
				.warn("first test warn")
				.debug("first test debug")
				.error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");
		}

		let got = log_out.lock().unwrap().clone();
		assert_eq!(got, tc.want, "{}", tc.name);
	}
}

#[test]
#[should_panic]
fn panic_no_sinks() {
	let mut log = slog::Slog::new();
	log.set_level(Level::Info).info("this should explode");
}

#[test]
#[should_panic]
fn panic_sink_after_async_sinks() {
	let mut log = slog::Slog::new();
	log.add_sink(sink::stdout::default()).set_async();
}

#[test]
#[should_panic]
fn panic_log_panics() {
	let mut log = slog::Slog::new();
	log.add_sink(sink::stdout::default()).set_level(Level::Info);

	log.info("this should log fine");
	log.panic("and this should explode");
}
