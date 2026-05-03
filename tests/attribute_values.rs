use rasant as r;
use rasant::sink;
use rasant::{Level, ToScalar, ToValue};

#[test]
fn methods() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	let test_keys = ["key_a".to_scalar(), "key_b".to_scalar(), "key_c".to_scalar()];
	let test_values = [123.to_scalar(), 456.to_scalar(), 789.to_scalar()];

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(string_sink);
		log.info_with("single value", [("result", (1234 as u32).to_value())]);
		log.info_with("list from array", [("result", test_values.to_value())]);
		log.info_with("list from slice", [("result", test_values.as_slice().to_value())]);
		log.info_with("map from arrays #1", [("map", [&test_keys, &test_values].to_value())]);
		log.info_with("map from arrays #2", [("map", (&test_keys, &test_values).to_value())]);
		log.info_with("map from slices #1", [("map", [test_keys.as_slice(), test_values.as_slice()].to_value())]);
		log.info_with("map from slices #2", [("map", (test_keys.as_slice(), test_values.as_slice()).to_value())]);
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] single value result=1234
2026-03-04 15:10:16.234 [INF] list from array result=[123, 456, 789]
2026-03-04 15:10:17.468 [INF] list from slice result=[123, 456, 789]
2026-03-04 15:10:18.702 [INF] map from arrays #1 map={\"key_a\": 123, \"key_b\": 456, \"key_c\": 789}
2026-03-04 15:10:19.936 [INF] map from arrays #2 map={\"key_a\": 123, \"key_b\": 456, \"key_c\": 789}
2026-03-04 15:10:21.170 [INF] map from slices #1 map={\"key_a\": 123, \"key_b\": 456, \"key_c\": 789}
2026-03-04 15:10:22.404 [INF] map from slices #2 map={\"key_a\": 123, \"key_b\": 456, \"key_c\": 789}";

	assert_eq!(got, want);
}

#[test]
fn macros() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	let test_keys = ["key_a", "key_b", "key_c"];
	let test_values = [123, 456, 789];

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(string_sink);
		r::info!(log, "single value", result = 1234);
		r::info!(log, "list from array", result = r::list!(test_values));
		// TODO: fix support for slices.
		//r::info!(log, "list from slice", result = r::list!(test_values.as_slice()));
		r::info!(log, "list from scalars", result = r::list!(123, 456.789, "lalala"));
		r::info!(log, "map from arrays", result = r::map!(test_keys, test_values));
		//r::info!(log, "map from slices", result = r::map!(test_keys.as_slice(), test_values.as_slice()));
		r::info!(log, "map from map!()", result = r::map!("key_a" => 123, 456 => 789.012, "key_c" => "string!"));
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] single value result=1234
2026-03-04 15:10:16.234 [INF] list from array result=[123, 456, 789]
2026-03-04 15:10:17.468 [INF] list from scalars result=[123, 456.789, \"lalala\"]
2026-03-04 15:10:18.702 [INF] map from arrays result={\"key_a\": 123, \"key_b\": 456, \"key_c\": 789}
2026-03-04 15:10:19.936 [INF] map from map!() result={\"key_a\": 123, 456: 789.012, \"key_c\": \"string!\"}";

	assert_eq!(got, want);
}
