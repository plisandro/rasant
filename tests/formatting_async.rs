use slog;
use slog::attributes::value::ToValue;
use slog::level::Level;
use slog::sink;

use std::io::{Error, ErrorKind};

#[test]
fn async_output() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = slog::Slog::new();
		log.set_level(Level::Info).add_sink(string_sink).set_async(true);
		log.info("root test info")
			.warn("root test warn")
			.fatal_with("oh no something horrible happened", [("why", "fire!".to_value())]);

		let mut nlog = log.clone();
		nlog.set("number", 1);
		nlog.info("first test info").warn("first test warn").error(Error::new(ErrorKind::NotFound, "oh no"), "something failed");
	}

	// collect result only after all loggers are dropped, as we'll race the output otherwise
	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] root test info
2026-03-04 15:10:16.234 [WRN] root test warn
2026-03-04 15:10:17.468 [FAT] oh no something horrible happened why=\"fire!\"
2026-03-04 15:10:18.702 [INF] first test info number=1
2026-03-04 15:10:19.936 [WRN] first test warn number=1
2026-03-04 15:10:21.170 [ERR] something failed error=\"oh no\" number=1";

	assert_eq!(got, want);
}
