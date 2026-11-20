#[cfg(test)]
pub mod sleep {
	use std::thread;
	use std::time;

	#[inline]
	pub fn millis(millis: u64) {
		thread::sleep(time::Duration::from_millis(millis));
	}
}
