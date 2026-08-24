use rasant as r;
use rasant::Level;
use rasant::Scalar;
use rasant::sink;

struct NameCard {
	name: &'static str,
	age: usize,
}

impl From<&NameCard> for Scalar {
	fn from(nc: &NameCard) -> Scalar {
		// note this will create a new String every time Dummy gets converted into an argument value
		Scalar::from(format!("i'm {name}, and i am {age} year old", name = nc.name, age = nc.age))
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
		let card = NameCard { name: "Rasant", age: 1 };

		let mut log = rasant::Logger::new();
		log.set_level(Level::Info).add_sink(mem_sink);

		r::info!(log, "hello!", hello_card = &card);
	}

	let got = mem_sink_output.as_string();
	let want = "2026-03-04 15:10:15.000 [INF] hello! hello_card=\"i\\'m Rasant, and i am 1 year old\"";

	assert_eq!(got, want);
}
