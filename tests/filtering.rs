use ntime;
use rasant as r;
use rasant::Level;
use rasant::filter;
use rasant::sink;

#[test]
fn step() {
	let mem_sink = sink::memory::Memory::new(sink::memory::MemoryConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::memory::MemoryConfig::default()
	});
	let mem_sink_output = mem_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info)
			.add_filter(filter::sample::Step::new(filter::sample::StepConfig { step: 7 }))
			.add_sink(mem_sink);

		for i in 0..40 {
			r::info!(log, "test info", iteration = i + 1);
			r::debug!(log, "i'm ignored :(");
		}
	}

	let got = mem_sink_output.as_string();
	let want = "2026-03-04 15:10:15.000 [INF] test info iteration=7
2026-03-04 15:10:16.234 [INF] test info iteration=14
2026-03-04 15:10:17.468 [INF] test info iteration=21
2026-03-04 15:10:18.702 [INF] test info iteration=28
2026-03-04 15:10:19.936 [INF] test info iteration=35";

	assert_eq!(got, want);
}

#[test]
fn burst() {
	let mem_sink = sink::memory::Memory::new(sink::memory::MemoryConfig {
		mock_time: true,
		mock_logger_id: true,
		..sink::memory::MemoryConfig::default()
	});
	let mem_sink_output = mem_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info)
			.add_filter(filter::sample::Burst::new(filter::sample::BurstConfig {
				period: ntime::Duration::from_millis(50),
				max_updates: 2,
			}))
			.add_sink(mem_sink);

		for i in 0..15 {
			r::info!(log, "test info", iteration = i + 1);
			r::debug!(log, "i'm ignored :(");
			ntime::sleep_millis(10);
		}
	}

	let got = mem_sink_output.as_string();
	let want = "2026-03-04 15:10:15.000 [INF] test info iteration=1
2026-03-04 15:10:16.234 [INF] test info iteration=2
2026-03-04 15:10:17.468 [INF] test info iteration=6
2026-03-04 15:10:18.702 [INF] test info iteration=7
2026-03-04 15:10:19.936 [INF] test info iteration=11
2026-03-04 15:10:21.170 [INF] test info iteration=12";

	assert_eq!(got, want);
}
