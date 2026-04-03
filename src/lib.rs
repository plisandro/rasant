//#![deny(missing_docs)]
#![allow(dead_code)]

mod attributes;
mod console;
mod level;
mod logger;
mod macros;
mod queue;
pub mod sink;

// Public exported symbols
pub use attributes::value::ToValue;
pub use level::Level;
pub use logger::Logger;
