use ntime::sleep_millis;
use rasant as r;
use rasant::FormatterConfig;
use rasant::Level;
use rasant::sink;

/// A Q&D dumb test to render a sample output for README.md.
#[test]
fn sample_log() {
	let mut log = rasant::Logger::new();

	let sink_a = sink::memory::Memory::new(sink::memory::MemoryConfig {
		formatter_cfg: FormatterConfig::default_color_compact(),
		..sink::memory::MemoryConfig::default()
	});
	let sink_a_output = sink_a.output();

	let sink_b = sink::memory::Memory::new(sink::memory::MemoryConfig {
		formatter_cfg: FormatterConfig::default_color_full(),
		..sink::memory::MemoryConfig::default()
	});
	let sink_b_output = sink_b.output();

	let sink_c = sink::memory::Memory::new(sink::memory::MemoryConfig {
		formatter_cfg: FormatterConfig::default_json(),
		..sink::memory::MemoryConfig::default()
	});
	let sink_c_output = sink_c.output();

	log.set_level(Level::Debug).add_sink(sink_a).add_sink(sink_b).add_sink(sink_c);

	{
		r::info!(log, "process start");
		sleep_millis(7);
		r::info!(log, "MyTestServer initialized", git_version = "3b683a11", config = "/etc/testserver.cfg");
		sleep_millis(11);
		let mut rlog = log.clone();
		rlog.set("id", 37);
		let mut clog = log.clone();
		clog.set("items", 1120213);
		r::info!(rlog, "REST server started", port = 443);
		sleep_millis(5);
		r::debug!(clog, "started cache backfill");
		sleep_millis(6);
		r::info!(rlog, "request", method = "GET", path = "/item/123", status = 200, time_ms = 37);
		sleep_millis(39);
		r::info!(rlog, "request", method = "POST", path = "/item/new");
		sleep_millis(28);

		let mut plog = rlog.clone();
		r::set!(plog, request_id = 329);
		r::debug!(plog, "wrote item to data store", store_id = 23995 as usize, status = 200);
		sleep_millis(23);
		r::info!(plog, "processed write", name = "new", time_ms = 187);
		sleep_millis(199);

		r::warn!(rlog, "request", method = "GET", path = "/item/a123", status = 403, time_ms = 29);
		sleep_millis(31);
		r::info!(rlog, "request", method = "GET", path = "/item/456", status = 200, time_ms = 19);
		sleep_millis(55);
		r::error!(clog, "cache backfill failed", done = 3493, error = "timeout reading from socket");
		sleep_millis(6);
	}

	println!("{}\n\n", sink_a_output.as_string());
	println!("{}\n\n", sink_b_output.as_string());
	println!("{}", sink_c_output.as_string());
}
