use std::process;

// TODO: fix me
pub fn process_info() -> (u32, Option<String>) {
	let pid = process::id();

	(pid, Some("process_name".into()))
}
