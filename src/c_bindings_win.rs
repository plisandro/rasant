use core::ffi::{c_char, c_int, c_ushort};
use std::ffi::CString;

const MAX_HOSTNAME_LENGTH: usize = 256;
const WSADATA_MAX_SIZE: usize = 64;
const WSADATA_VERSION: u16 = 0x22; // Windows Sockets version 2.2.

/* ----------------------- Bindings for MSVC functions ----------------------- */

// MSVC C standard functions
unsafe extern "C" {
	// https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-gethostname
	unsafe fn gethostname(name: *mut c_char, namelen: c_int) -> c_int;
	// https://learn.microsoft.com/en-us/windows/win32/api/winsock/nf-winsock-wsastartup
	unsafe fn WSAStartup(wVersionRequested: c_ushort, lpWSAData: *mut c_char) -> c_int;
}

/* -----------------------Safe C function wrappers ----------------------- */

// Initializes Windows's sockets.
fn c_wsa_startup() {
	// dummy buffer for WSADATA struct, which we don't care one bit about.
	// see https://learn.microsoft.com/en-us/windows/win32/api/winsock/ns-winsock-wsadata for details.
	let mut dummy_buf: Vec<u8> = vec![0; WSADATA_MAX_SIZE];

	// SAFETY: Calling MSVC functions with properly initialized types.
	unsafe {
		let err = WSAStartup(WSADATA_VERSION as c_ushort, dummy_buf.as_mut_ptr() as *mut c_char);
		if err != 0 {
			panic!("WSAStartup() failed with error {err}");
		}
	}
}

// Resolves the system's hostname, if available.
pub fn c_get_hostname() -> Option<String> {
	let res: Option<String>;

	c_wsa_startup();

	// SAFETY: Calling MSVC functions with properly initialized types.
	unsafe {
		let c_hostname_buf: Vec<u8> = vec![0; MAX_HOSTNAME_LENGTH];
		let c_hostname = CString::from_vec_unchecked(c_hostname_buf);
		let c_hostname_ptr = c_hostname.into_raw();

		if gethostname(c_hostname_ptr as *mut c_char, MAX_HOSTNAME_LENGTH as c_int) != 0 {
			return None;
		}

		res = match CString::from_raw(c_hostname_ptr).into_string() {
			Ok(s) => Some(s),
			Err(_) => None,
		}
	}

	res
}
