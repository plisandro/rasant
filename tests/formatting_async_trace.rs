use ntime;
use rasant::Level;
use rasant::ToValue;
use rasant::sink;

use std::io::{Error, ErrorKind};

#[test]
fn async_trace() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Trace).add_sink(string_sink).set_async(true);
		log.info("root test info")
			.warn("root test warn")
			.fatal_with("oh no something horrible happened", [("why", "fire!".to_value())]);

		let mut nlog = log.clone();
		nlog.set("number", 1);
		nlog.info("first test info").warn("first test warn").error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");

		// give a little time for all logs to flush before we drop loggers, which produce traces
		ntime::sleep_millis(10);
	}

	// collect result only after all loggers are dropped, as we'll race the output otherwise
	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [TRA] added new log sink name=\"default log string\" async=false logs_all_levels=false logger_id=100
2026-03-04 15:10:16.234 [TRA] enabled async log updates total_async_loggers=1 logger_id=101
2026-03-04 15:10:17.468 [INF] root test info
2026-03-04 15:10:18.702 [WRN] root test warn
2026-03-04 15:10:19.936 [FAT] oh no something horrible happened why=\"fire!\"
2026-03-04 15:10:21.170 [TRA] enabled async log updates total_async_loggers=2 logger_id=102
2026-03-04 15:10:22.404 [INF] first test info number=1
2026-03-04 15:10:23.638 [WRN] first test warn number=1
2026-03-04 15:10:24.872 [ERR] something failed error=\"oh no\" number=1
2026-03-04 15:10:26.106 [TRA] disabled async log updates number=1 total_async_loggers=1 logger_id=103
2026-03-04 15:10:27.340 [TRA] disabled async log updates total_async_loggers=0 logger_id=104";

	assert_eq!(got, want);
}
