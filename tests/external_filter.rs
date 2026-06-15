use rasant as r;
use rasant::Level;
use rasant::filter;
use rasant::sink;

struct DummyFilter {
	count: usize,
}

impl DummyFilter {
	pub fn new() -> Self {
		Self { count: 0 }
	}
}

impl filter::Filter for DummyFilter {
	fn name(&self) -> &str {
		"a dumb test filter"
	}

	fn pass<'f>(&mut self, _: &'f sink::LogUpdate) -> bool {
		self.count += 1;
		self.count % 2 == 0
	}
}

#[test]
fn external_sink() {
	let mem_sink = sink::memory::Memory::new(sink::memory::MemoryConfig {
		mock_time: true,
		..sink::memory::MemoryConfig::default()
	});
	let mem_sink_output = mem_sink.output();

	{
		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(mem_sink);
		log.add_filter(DummyFilter::new());

		r::info!(log, "this", num = 1);
		r::info!(log, "should", num = 2);
		r::info!(log, "only", num = 3);
		r::info!(log, "log", num = 4);
		r::info!(log, "even", num = 5);
		r::info!(log, "entries", num = 6);
	}

	let got = mem_sink_output.as_string();
	let want = "2026-03-04 15:10:15.000 [INF] should num=2\n\
	            2026-03-04 15:10:16.234 [INF] log num=4\n\
				2026-03-04 15:10:17.468 [INF] entries num=6";

	assert_eq!(got, want);
}
