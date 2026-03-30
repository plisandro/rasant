mod attributes;
mod console;
mod level;
mod logger;
mod macros;
mod queue;
pub mod sink;
// TODO: replace me with ntime
pub mod time;

//#![deny(missing_docs)]
//#![allow(dead_code)]

pub use attributes::value::ToValue;
pub use level::Level;
pub use logger::Logger;
