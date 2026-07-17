use rasant as r;
use rasant::Level;
use rasant::sink;

/// Tests different macro call syntaxes
#[test]
fn macro_calls() {
	let mut log = r::Logger::new();
	log.set_level(Level::Info).add_sink(sink::black_hole::default());

	// logger as parameter
	r::info!(log, "parameter");
	r::info!(log, "parameter", with = "argument");

	// logger as reference
	let rlog = &mut log;

	r::info!(rlog, "reference");
	r::info!(rlog, "reference", with = "argument");

	// loger as struct field
	struct TestStruct {
		log: r::Logger,
	}
	let mut ts = TestStruct { log: log.clone() };

	r::info!(ts.log, "struct field");
	r::info!(ts.log, "struct field", with = "argument");

	// logger as struct reference field
	let rts = &mut ts;

	r::info!(rts.log, "struct reference field");
	r::info!(rts.log, "struct reference field", with = "argument");
}

/// Test the behavior of logging macros with different message input types.
#[test]
fn macro_msg_input() {
	let mem_sink = sink::memory::Memory::new(sink::memory::MemoryConfig {
		mock_time: true,
		..sink::memory::MemoryConfig::default()
	});
	let mem_sink_output = mem_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(mem_sink);
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
	let got = mem_sink_output.as_string();
	let want = "2026-03-04 15:10:15.000 [INF] root test info\n\
	            2026-03-04 15:10:16.234 [WRN] a root test warn from String\n\
				2026-03-04 15:10:17.468 [FAT] oh no something horrible happened why=\"fire\"\n\
				2026-03-04 15:10:18.702 [INF] first test info number=1\n\
				2026-03-04 15:10:19.936 [WRN] first test warn number=1";

	assert_eq!(got, want);
}
