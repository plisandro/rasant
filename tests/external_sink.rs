use std::io;
use std::io::Write;
use std::sync::Mutex;

use rasant as r;
use rasant::Level;
use rasant::sink;

static SINK_OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

struct DummySink {}

impl sink::Sink for DummySink {
	fn name(&self) -> &str {
		"a dumb test sink"
	}

	fn log<'f>(&mut self, update: &'f sink::LogUpdate) -> io::Result<()> {
		let mut out = SINK_OUTPUT.lock().unwrap();

		write!(out, "level: {:?}, msg: {}, attrs:", update.level(), update.message())?;
		for (key, value, meta) in update.attributes().iter() {
			write!(out, " <{} (metadata 0x{:08b}) -> {}>", key, meta, value)?;
		}
		out.push('\n' as u8);

		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

#[test]
fn external_sink() {
	let sink = DummySink {};
	let mut log = rasant::Logger::new();
	log.set_level(Level::Info).add_sink(sink);

	r::info!(log, "single value", result = 1234);
	r::info!(log, "a list", result = r::list!([1, 2, 3, 4]));
	r::debug!(log, "i'm not logged at all :(");

	let got = String::from_utf8(SINK_OUTPUT.lock().unwrap().clone()).expect("invalid UTF-8 contents for dummy sink");
	let want = "level: Info, msg: single value, attrs: <result (metadata 0x00000000) -> 1234>\n\
	            level: Info, msg: a list, attrs: <result (metadata 0x00000000) -> [1, 2, 3, 4]>\n";

	assert_eq!(got, want);
}
