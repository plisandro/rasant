use std::env;
use std::fs;

use std::path::PathBuf;

use ntime;
use rasant as r;
use rasant::sink;
use rasant::{FormatterConfig, Level};

fn test_filename() -> PathBuf {
	let mut path = env::temp_dir();
	let now = ntime::Timestamp::now();
	path.push(format!("file_sink_test_{ts}.log", ts = now.as_nanos()));
	return path;
}

fn read_file(path: &PathBuf) -> String {
	let content = fs::read_to_string(path).expect("failed to read test file");
	// read_line() insists on adding newlines even when they're not there >:(
	return content.trim().into();
}

#[test]
fn append() {
	let log_file_path = test_filename();
	let mut want = String::new();

	{
		// First pass, write to file + string
		let mut log = r::Logger::new();
		let string_sink = sink::string::String::new(sink::string::StringConfig {
			formatter_cfg: FormatterConfig::default(),
			..sink::string::StringConfig::default()
		});
		let string_out = string_sink.output();

		log.set_level(Level::Info).add_sink(string_sink);
		log.add_sink(sink::file::new(sink::file::FileConfig {
			path: Some(log_file_path.clone()),
			formatter_cfg: FormatterConfig::default(),
			append: true,
			..sink::file::FileConfig::default()
		}));
		r::set!(log, pass = 1);

		r::info!(log, "test info");
		r::debug!(log, "i'm ignored :(");
		r::warn!(log, "test warn");
		r::fatal!(log, "oh no something horrible happened", what = "fire!");

		want.push_str(&string_out.lock().unwrap());
		// Account for file sinks adding a delimiter on append.
		want.push_str("\n");
	}

	{
		// Second pass, write to file + string
		let mut log = r::Logger::new();
		let string_sink = sink::string::String::new(sink::string::StringConfig {
			formatter_cfg: FormatterConfig::default(),
			..sink::string::StringConfig::default()
		});
		let string_out = string_sink.output();

		log.set_level(Level::Info).add_sink(string_sink);
		log.add_sink(sink::file::new(sink::file::FileConfig {
			path: Some(log_file_path.clone()),
			formatter_cfg: FormatterConfig::default(),
			append: true,
			..sink::file::FileConfig::default()
		}));
		r::set!(log, pass = 2);

		r::info!(log, "test info");
		r::debug!(log, "i'm ignored :(");
		r::warn!(log, "test warn");
		r::fatal!(log, "oh no something horrible happened", what = "fire!");

		want.push_str(&string_out.lock().unwrap());
	}

	let got = read_file(&log_file_path);
	assert_eq!(got, want, "check log file contents after append write");

	fs::remove_file(log_file_path).expect("failed to delete test file");
}

#[test]
fn overwrite() {
	let log_file_path = test_filename();
	let want: String;

	{
		// First pass, write to file only
		let mut log = r::Logger::new();
		log.set_level(Level::Info).add_sink(sink::file::new(sink::file::FileConfig {
			path: Some(log_file_path.clone()),
			formatter_cfg: FormatterConfig::default(),
			append: false,
			..sink::file::FileConfig::default()
		}));
		r::set!(log, pass = 1);

		r::info!(log, "test info");
		r::debug!(log, "i'm ignored :(");
		r::warn!(log, "test warn");
		r::fatal!(log, "oh no something horrible happened", what = "fire!");
	}

	{
		// Second pass, write to file + string
		let mut log = r::Logger::new();
		let string_sink = sink::string::String::new(sink::string::StringConfig {
			formatter_cfg: FormatterConfig::default(),
			..sink::string::StringConfig::default()
		});
		let string_out = string_sink.output();

		log.set_level(Level::Info).add_sink(string_sink);
		log.add_sink(sink::file::new(sink::file::FileConfig {
			path: Some(log_file_path.clone()),
			formatter_cfg: FormatterConfig::default(),
			append: false,
			..sink::file::FileConfig::default()
		}));
		r::set!(log, pass = 2);

		r::info!(log, "test info");
		r::debug!(log, "i'm ignored :(");
		r::warn!(log, "test warn");
		r::fatal!(log, "oh no something horrible happened", what = "fire!");

		want = string_out.lock().unwrap().clone();
	}

	let got = read_file(&log_file_path);
	assert_eq!(got, want, "check log file contents after truncate write");

	fs::remove_file(log_file_path).expect("failed to delete test file");
}
