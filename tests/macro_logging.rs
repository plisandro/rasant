use rasant as r;
use rasant::sink;

use std::io::{Error, ErrorKind};

#[test]
fn macro_logging() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(rasant::Level::Trace).add_sink(string_sink);

		r::info!(log, "root test, info without args");
		r::info!(log, "root test, info with args", first = 1234, second = "lala");
		r::warn!(log, "root test, warn");
		r::debug!(log, "root test, debug", a_float = 3.1415926);

		let mut nlog = log.clone();
		r::set!(nlog, number = 1);
		r::info!(nlog, "first test info");
		r::warn!(nlog, "first test warn", warning = "fire!");
		r::debug!(nlog, "first test debug");
		r::error!(nlog, Error::new(ErrorKind::NotFound, "oh no"), "something failed");
		r::error!(nlog, Error::new(ErrorKind::InvalidInput, "again!"), "another error", with = "attributes");
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [TRA] added new log sink name=\"default log string\" total=1 async=false logger_id=100
2026-03-04 15:10:16.234 [INF] root test, info without args
2026-03-04 15:10:17.468 [INF] root test, info with args first=1234 second=\"lala\"
2026-03-04 15:10:18.702 [WRN] root test, warn
2026-03-04 15:10:19.936 [DBG] root test, debug a_float=3.1415926
2026-03-04 15:10:21.170 [INF] first test info number=1
2026-03-04 15:10:22.404 [WRN] first test warn number=1 warning=\"fire!\"
2026-03-04 15:10:23.638 [DBG] first test debug number=1
2026-03-04 15:10:24.872 [ERR] something failed error=\"oh no\" number=1
2026-03-04 15:10:26.106 [ERR] another error error=\"again!\" number=1 with=\"attributes\"";

	assert_eq!(got, want);
}

#[test]
#[should_panic]
fn panic_logging() {
	let mut log = rasant::Logger::new();
	log.add_sink(sink::stdout::default()).set_level(rasant::Level::Info);

	r::info!(log, "this should work");
	r::panic!(log, "and this should panic!");
}
