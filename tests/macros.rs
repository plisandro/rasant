use rasant as r;
use rasant::Level;
use rasant::sink;

/// Test the behavior of logging macros with different message input types.
#[test]
fn macro_msg_input() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(string_sink);
		r::info!(log, "root test info");
		r::warn!(log, format!("a {a} test warn from {b}", a = "root", b = "String").as_str());
		r::fatal!(log, "oh no something horrible happened", why = "fire");

		let mut nlog = log.clone();
		nlog.set("number", 1);
		let info_msg: &str = "first test info";
		r::info!(nlog, info_msg);
		r::warn!(nlog, String::from("first test warn").as_str());
	}

	// collect result only after all loggers are dropped, as we'll race the output otherwise
	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] root test info
2026-03-04 15:10:16.234 [WRN] a root test warn from String
2026-03-04 15:10:17.468 [FAT] oh no something horrible happened why=\"fire\"
2026-03-04 15:10:18.702 [INF] first test info number=1
2026-03-04 15:10:19.936 [WRN] first test warn number=1";

	assert_eq!(got, want);
}
