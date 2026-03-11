use slog;
use slog::level::Level;
use slog::sink;
use slog::time::{StringFormat, Timestamp};

use std::io::{Error, ErrorKind};
use std::sync::Mutex;

#[test]
fn log_to_string() {
	const WANT: &str = "2026-03-04 15:10:15.000 [INF] log level updated name=\"info\" new_level=2
2026-03-04 15:10:16.234 [INF] root test info
2026-03-04 15:10:17.468 [WRN] root test warn
2026-03-04 15:10:18.702 [INF] first test info number=1
2026-03-04 15:10:19.936 [WRN] first test warn number=1
2026-03-04 15:10:21.170 [ERR] something failed error=\"oh no\" number=1";

	for set_async in [false /* , true*/] {
		let mut log_out = Mutex::new(String::from(""));

		{
			let mut log = slog::Slog::new();
			if set_async {
				log.set_async();
			}
			log.add_sink(sink::string::String::new(sink::string::StringConfig {
				out: Some(&log_out),
				mock_time: true,
				..sink::string::StringConfig::default()
			}))
			.add_sink(sink::stdout::default())
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
		assert_eq!(got, WANT, "async={}", set_async);
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
