use ntime;
use rasant as r;
use rasant::Level;
use rasant::filter;
use rasant::sink;

#[test]
fn step() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info)
			.add_filter(filter::sample::Step::new(filter::sample::StepConfig { step: 7 }))
			.add_sink(string_sink);

		for i in 0..40 {
			r::info!(log, "test info", iteration = i + 1);
			r::debug!(log, "i'm ignored :(");
		}
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] test info iteration=7
2026-03-04 15:10:16.234 [INF] test info iteration=14
2026-03-04 15:10:17.468 [INF] test info iteration=21
2026-03-04 15:10:18.702 [INF] test info iteration=28
2026-03-04 15:10:19.936 [INF] test info iteration=35";

	assert_eq!(got, want);
}

#[test]
fn burst() {
	let string_sink = sink::string::String::new(sink::string::StringConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::string::StringConfig::default()
	});
	let string_sink_output = string_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info)
			.add_filter(filter::sample::Burst::new(filter::sample::BurstConfig {
				period: ntime::Duration::from_millis(5),
				max_messages: 2,
			}))
			.add_sink(string_sink);

		for i in 0..15 {
			r::info!(log, "test info", iteration = i + 1);
			r::debug!(log, "i'm ignored :(");
			ntime::sleep_millis(1);
		}
	}

	let got = string_sink_output.lock().unwrap().clone();
	let want = "2026-03-04 15:10:15.000 [INF] test info iteration=1
2026-03-04 15:10:16.234 [INF] test info iteration=2
2026-03-04 15:10:17.468 [INF] test info iteration=6
2026-03-04 15:10:18.702 [INF] test info iteration=7
2026-03-04 15:10:19.936 [INF] test info iteration=11
2026-03-04 15:10:21.170 [INF] test info iteration=12";

	assert_eq!(got, want);
}
