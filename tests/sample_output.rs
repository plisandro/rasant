use ntime::sleep_millis;
use rasant as r;
use rasant::Level;
use rasant::sink;
use rasant::sink::format::FormatterConfig;

/// A Q&D dumb test to render a sample output for README.md.
#[test]
fn sample_log() {
	let mut log = rasant::Logger::new();

	let sink_a = sink::string::String::new(sink::string::StringConfig {
		formatter_cfg: FormatterConfig::color(),
		..sink::string::StringConfig::default()
	});
	let sink_a_output = sink_a.output();

	let sink_b = sink::string::String::new(sink::string::StringConfig {
		formatter_cfg: FormatterConfig::json(),
		..sink::string::StringConfig::default()
	});
	let sink_b_output = sink_b.output();

	log.set_level(Level::Debug).add_sink(sink_a).add_sink(sink_b);

	{
		r::info!(log, "MyTestServer initialized", git_version = "3b683a11", config = "/etc/testserver.cfg");
		sleep_millis(11);
		let mut rlog = log.clone();
		rlog.set("thread_id", 37);
		let mut clog = log.clone();
		clog.set("total_items", 1120219033);
		r::info!(rlog, "REST server started", port = 8080);
		sleep_millis(5);
		r::debug!(clog, "started cache backfill");
		sleep_millis(6);
		r::info!(
			rlog,
			"got request",
			method = "GET",
			path = "/item/123",
			address = "172.66.174.159:443",
			status = 200,
			response_time_ms = 37
		);
		sleep_millis(39);
		r::info!(
			rlog,
			"got request",
			method = "POST",
			path = "/item/new",
			address = "199.232.16.176:443",
			status = 200,
			response_time_ms = 265
		);
		sleep_millis(268);
		r::warn!(
			rlog,
			"invalid request",
			method = "GET",
			path = "/item/a123",
			address = "104.20.20.242:443",
			status = 403,
			response_time_ms = 29
		);
		sleep_millis(31);
		r::info!(
			rlog,
			"got request",
			method = "GET",
			path = "/item/456",
			address = "2001:4860:482d:77:443",
			status = 200,
			response_time_ms = 19
		);
		sleep_millis(55);
		r::error!(clog, "cache backfill failed", items = 349303, error = "redis.exceptions.TimeoutError: Timeout reading from socket");
		sleep_millis(6);
	}

	println!("{}\n", sink_a_output.lock().unwrap());
	println!("{}\n", sink_b_output.lock().unwrap());
}
