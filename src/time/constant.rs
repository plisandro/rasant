pub const TIMEZONE_UTC: &str = "UTC";

// dumb conversion constants, but these *really* help redability
pub const U32_MILLIS_IN_SECOND: u32 = 1000;
pub const U32_NANOS_IN_MILLI: u32 = 1000 * 1000;
pub const U32_NANOS_IN_SECOND: u32 = U32_MILLIS_IN_SECOND * U32_NANOS_IN_MILLI;
pub const U64_MILLIS_IN_SECOND: u64 = U32_MILLIS_IN_SECOND as _;
pub const U64_NANOS_IN_MILLI: u64 = U32_NANOS_IN_MILLI as _;
pub const U128_MILLIS_IN_SECOND: u128 = U32_MILLIS_IN_SECOND as _;
pub const U128_NANOS_IN_MILLI: u128 = U32_NANOS_IN_MILLI as _;
pub const U128_NANOS_IN_SECOND: u128 = U32_NANOS_IN_SECOND as _;
