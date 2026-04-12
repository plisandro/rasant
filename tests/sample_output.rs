use ntime::sleep_millis;
use rasant as r;
use rasant::FormatterConfig;
use rasant::Level;
use rasant::sink;

/// A Q&D dumb test to render a sample output for README.md.
#[test]
fn sample_log() {
	let mut log = rasant::Logger::new();

	let sink_a = sink::string::String::new(sink::string::StringConfig {
		formatter_cfg: FormatterConfig::default_color(),
		..sink::string::StringConfig::default()
	});
	let sink_a_output = sink_a.output();

	let sink_b = sink::string::String::new(sink::string::StringConfig {
		formatter_cfg: FormatterConfig::default_json(),
		..sink::string::StringConfig::default()
	});
	let sink_b_output = sink_b.output();

	log.set_level(Level::Debug).add_sink(sink_a).add_sink(sink_b);

	{
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
		r::info!(rlog, "request", method = "POST", path = "/item/new", status = 200, time_ms = 265);
		sleep_millis(268);
		r::warn!(rlog, "request", method = "GET", path = "/item/a123", status = 403, time_ms = 29);
		sleep_millis(31);
		r::info!(rlog, "request", method = "GET", path = "/item/456", status = 200, time_ms = 19);
		sleep_millis(55);
		r::error!(clog, "cache backfill failed", done = 3493, error = "timeout reading from socket");
		sleep_millis(6);
	}

	println!("{}\n\n", sink_a_output.lock().unwrap());
	println!("{}", sink_b_output.lock().unwrap());
}
