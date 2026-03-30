use rasant::Level;
use rasant::sink;
use rasant::*;

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
		log.set_level(Level::Trace).add_sink(string_sink);

		//log.info("root test info").warn("root test warn").debug("root test debug");
		info!(log, "root test, info without args");
		info!(log, "root test, info with args", "first" = 1234, "second" = "lala");
		warn!(log, "root test, warn");
		debug!(log, "root test, debug", "a_float" = 3.1415926);

		let mut nlog = log.clone();
		nlog.set("number", 1);
		info!(nlog, "first test info");
		warn!(nlog, "first test warn", "waring" = "fire!");
		debug!(nlog, "first test debug");
		error!(nlog, Error::new(ErrorKind::NotFound, "oh no"), "something failed");
		error!(nlog, Error::new(ErrorKind::InvalidInput, "again!"), "another error", "with" = "attributes");
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [TRA] added new log sink name=\"default log string\" async=false logs_all_levels=false logger_id=100
2026-03-04 15:10:16.234 [INF] root test, info without args
2026-03-04 15:10:17.468 [INF] root test, info with args first=1234 second=\"lala\"
2026-03-04 15:10:18.702 [WRN] root test, warn
2026-03-04 15:10:19.936 [DBG] root test, debug a_float=3.1415926
2026-03-04 15:10:21.170 [INF] first test info number=1
2026-03-04 15:10:22.404 [WRN] first test warn number=1 waring=\"fire!\"
2026-03-04 15:10:23.638 [DBG] first test debug number=1
2026-03-04 15:10:24.872 [ERR] something failed error=\"oh no\" number=1
2026-03-04 15:10:26.106 [ERR] another error error=\"again!\" number=1 with=\"attributes\"";

	assert_eq!(got, want);
}
