use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int};

#[cfg(not(target_env = "msvc"))]
use std::os::raw::c_long;

/* ----------------------- Bindings for C stdlib time functions ----------------------- */

// time_t is platform-specific, so use the largest type available
pub type CTime = u64;
#[cfg(target_env = "msvc")]
pub type CErrno = c_char;

// *nix timezone fields
#[cfg(not(target_env = "msvc"))]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct c_tm {
	pub tm_sec: c_int,
	pub tm_min: c_int,
	pub tm_hour: c_int,
	pub tm_mday: c_int,
	pub tm_mon: c_int,
	pub tm_year: c_int,
	pub tm_wday: c_int,
	pub tm_yday: c_int,
	pub tm_isdst: c_int,
	pub tm_gmtoff: c_long,
	pub tm_zone: *mut c_char,
}

// Windows MSVC timezone fields
#[cfg(target_env = "msvc")]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct c_tm {
	pub tm_sec: c_int,
	pub tm_min: c_int,
	pub tm_hour: c_int,
	pub tm_mday: c_int,
	pub tm_mon: c_int,
	pub tm_year: c_int,
	pub tm_wday: c_int,
	pub tm_yday: c_int,
	pub tm_isdst: c_int,
}

// *nix C standard time functions
#[cfg(not(target_env = "msvc"))]
unsafe extern "C" {
	unsafe fn gmtime_r(ts: *const CTime, tm: *mut c_tm) -> *mut c_tm;
	unsafe fn localtime_r(ts: *const CTime, tm: *mut c_tm) -> *mut c_tm;
	unsafe fn timegm(tm: *mut c_tm) -> CTime;
}

// Windows MSVC standard time functions
#[cfg(target_env = "msvc")]
unsafe extern "C" {
	unsafe fn _gmtime64_s(tm: *mut c_tm, ts: *const CTime) -> c_int;
	unsafe fn _localtime64_s(tm: *mut c_tm, ts: *const CTime) -> c_int;
	unsafe fn _mkgmtime64(tm: *mut c_tm) -> CTime;
	// Windows is stupid and doesn't return TZ information in tm structs, so...
	unsafe fn _get_timezone(seconds: *mut s) -> CErrno;
	unsafe fn _get_tzname(pReturnValue: *mut c_size_t, timeZoneName: *mut c_char, sizeInBytes: *mut c_size_t, index: *mut c_int) -> CErrno;
}

// Safe C function wrappers
pub fn c_time_to_utc_tm(ts: CTime) -> Option<c_tm> {
	let ok: bool;
	let ts: *const CTime = &ts;
	let mut tm = MaybeUninit::<c_tm>::uninit();

	unsafe {
		#[cfg(not(target_env = "msvc"))]
		{
			ok = !gmtime_r(ts, tm.as_mut_ptr()).is_null();
		}
		#[cfg(target_env = "msvc")]
		{
			ok = _gmtime64_s(tm.as_mut_ptr(), ts) == 0;
		}
	}
	if !ok {
		return None;
	}

	let tm = unsafe { tm.assume_init() };
	Some(tm)
}

pub fn c_time_to_local_tm(ts: CTime) -> Option<c_tm> {
	let ok: bool;
	let ts: *const CTime = &ts;
	let mut tm = MaybeUninit::<c_tm>::uninit();

	unsafe {
		#[cfg(not(target_env = "msvc"))]
		{
			ok = !localtime_r(ts, tm.as_mut_ptr()).is_null();
		}
		#[cfg(target_env = "msvc")]
		{
			ok = _localtime64_s(tm.as_mut_ptr(), ts) == 0;
		}
	}
	if !ok {
		return None;
	}

	let tm = unsafe { tm.assume_init() };
	Some(tm)
}

pub fn c_utc_tm_to_time(tm: &mut c_tm) -> CTime {
	let ct: CTime;
	let tm: *mut c_tm = tm;

	unsafe {
		#[cfg(not(target_env = "msvc"))]
		{
			ct = timegm(tm);
		}
		#[cfg(target_env = "msvc")]
		{
			ct = _mkgtime64(tm);
		}
	}

	ct
}

#[cfg(target_env = "msvc")]
pub fn c_tz_info() -> (&string, i16) {
	todo!("TZ information support for Windows is not yet implemented");
	("UTC", 0)
}
