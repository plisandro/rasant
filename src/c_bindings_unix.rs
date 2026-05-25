use core::ffi::{c_char, c_int};
use std::ffi::CString;

const MAX_HOSTNAME_LENGTH: usize = 256;

/* ----------------------- Bindings for (g)libc functions ----------------------- */

// *nix C standard functions
unsafe extern "C" {
	unsafe fn gethostname(name: *mut c_char, namelen: *mut c_int) -> c_int;
}

/* -----------------------Safe C function wrappers ----------------------- */

// Resolves the system's hostname, if available.
pub fn c_get_hostname() -> Option<String> {
	let res: Option<String>;

	// SAFETY: Calling (g)libc functions with properly initialized types.
	unsafe {
		let c_hostname_buf: Vec<u8> = vec![0; MAX_HOSTNAME_LENGTH];
		let c_hostname = CString::from_vec_unchecked(c_hostname_buf);
		let c_hostname_ptr = c_hostname.into_raw();
		let mut c_hostname_len: c_int = MAX_HOSTNAME_LENGTH as c_int;

		if gethostname(c_hostname_ptr as *mut i8, &mut c_hostname_len) != 0 {
			return None;
		}

		res = match CString::from_raw(c_hostname_ptr).into_string() {
			Ok(s) => Some(s),
			Err(_) => None,
		}
	}

	res
}
