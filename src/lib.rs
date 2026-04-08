//! Rasant is a lightweight, high performance and flexible Rust library for structured logging,
//! inspired by the likes of [zap](https://github.com/uber-go/zap) and [zerolog](https://github.com/rs/zerolog).
//!
//! It offers nanosecond precision, stackable logging and outstanding performance: on modern
//! systems, Rasant can process and dispatch logs to multiple sinks in tens of nanoseconds, being
//! normally bottlenecked by I/O operations. Can't wait that long? There's built-in async support!
//!
//! # Main Features
//!
//!   - Minimal dependencies.
//!   - *Blazing fast* performance, with zero allocations on most operations.
//!   - [Leveled][`Level`], structured contextual logging with [nanosecond precision](https://crates.io/crates/ntime).
//!   - [Simple API](#basic-logging), with support for [stacked logging](#stacking).
//!   - Thread safe.
//!   - [Highly configurable log sinks](#configuring-sinks).
//!   - Text and JSON log output.
//!   - Support for [dynamic async logging](#asynchronous-logging) with constant lock time.
//!
//! # Examples
//!
//! # Basic Logging
//!
//! [`Logger`]s can be easily initialized using sink defaults, and accessed via methods...
//!
//! ```rust
//! use rasant;
//! use rasant::ToValue;
//!
//! let mut log = rasant::Logger::new();
//! log.add_sink(rasant::sink::stderr::default()).set_level(rasant::Level::Info);
//!
//! log.set("program_name", "test");
//! log.info("hello world!");
//! log.warn_with("here's some context", [("line", 7.to_value())]);
//! log.debug("and i'm ignored :(");
//! ```
//!
//! ...or the _much_ nicer macro API:
//!
//! ```rust
//! use rasant as r;
//!
//! let mut log = r::Logger::new();
//! log.add_sink(r::sink::stderr::default()).set_level(r::Level::Info);
//!
//! r::set!(log, program_name="test");
//! r::info!(log, "hello world!");
//! r::warn!(log, "here's some context", line = 7);
//! r::debug!(log, "and i'm ignored :(");
//! ```
//!
//! ```text
//! 2026-04-03 17:16:03.773 +0200 [INF] hello world! program_name="test"
//! 2026-04-03 17:16:03.773 +0200 [WRN] here's some context program_name="test" line=7
//! ```
//!
//! ## Stacking
//!
//! All [`Logger`]s can be cheaply cloned, inheriting all settings from its
//! parent - including [levels][`level::Level`], [Sink][`sink::Sink`]s and fixed
//! [attributes](#attributes), allowing for very flexible  setups.
//!
//! For example, to have all errors (or higher) within a thread logged to
//! `stderr`:
//!
//! ```rust
//! use rasant as r;
//! use std::thread;
//!
//! let mut log = r::Logger::new();
//! log.add_sink(r::sink::stdout::default()).set_level(r::Level::Info);
//! r::info!(log, "main logs to stdout only");
//!
//! let mut thread_log = log.clone();
//! thread::spawn(move || {
//!     thread_log.add_sink(r::sink::stderr::default()).set_level(r::Level::Error);
//!
//! 	r::set!(thread_log, thread_id = thread::current().id());
//! 	r::info!(thread_log, "this will not log anything");
//! 	r::fatal!(thread_log, "but this will log to both stdout and stderr");
//! });
//! ```
//!
//! ## Configuring Sinks
//!
//! [Sink][`sink::Sink`]s can be configured to tweak multiple parameters, including time and
//! overall output format.
//!
//! ```rust
//! use ntime;
//! use rasant as r;
//!
//! let mut log = r::Logger::new();
//! log.set_level(r::Level::Info).add_sink(
//!     r::sink::stdout::new(r::sink::stdout::StdoutConfig {
//! 		formatter_cfg: r::FormatterConfig {
//! 			format: r::OutputFormat::Json,
//! 			time_format: ntime::Format::UtcNanosDateTime,
//! 			..r::FormatterConfig::default()
//! 		},
//! 		..r::sink::stdout::StdoutConfig::default()
//! 	})
//! );
//!
//! r::info!(log, "hello!");
//! ```
//!
//! ```text
//! {"time":"2026-04-03 16:03:04.481888522","level":"info","message":"hello!"}
//! ```
//!
//! ## Asynchronous Logging
//!
//! [`Logger`]s can dynamically enable/disable async writes.
//!
//! When in async mode, log operations have a slightly longer (as details
//! are copied into a queue) _but fixed_ lock time, making it ideal f.ex.
//! for writing to slow storage without compromising overall performance.
//!
//! ```rust
//! use rasant as r;
//!
//! let mut log = r::Logger::new();
//! log.set_level(r::Level::Info).add_sink(r::sink::stdout::default());
//!
//! r::info!(log, "this is writen synchronously");
//! log.set_async(true);
//! r::info!(log, "and these write");
//! r::warn!(log, "asynchronously, but");
//! r::info!(log, "in order!");
//! ```
//!
//! # Concepts
//!
//! Rasant is a structured logging library: it logs messages with a set of associated key-[`Value`]
//! pairs, in formats (f.ex. [JSON][`OutputFormat::Json`]) which are intended to
//! be easily machine-readable.
//!
//! ## Methodology
//!
//! Rasant is built around individual [`Logger`] logging instances and [Sink][`sink::Sink`]s, which are
//! configurable destinations for log updates. When a log operation is performed, its level
//! is compared to the one defined for the [`Logger`] and, if applicable, the log is written
//! on all its [Sink][`sink::Sink`]s.
//!
//! Once a [Sink][`sink::Sink`] is added to a [`Logger`], it cannot be removed nor modified.
//!
//! ## Attributes
//!
//! Attributes are the defining quality of a structured logging system, expressed
//! as key-value pairs. Keys are [`&str`], whereas [`Value`]s are a set of
//! fixed internal types, which can be easily instantiated from common Rust types and structs.
//!
//! ### Scope
//!
//! Attributes can be set for [`Logger`]s as a whole, affecting all log operations, or for
//! individual log writes, which can optionally provide extra attributes with additional
//! information.
//!
//! Upon key collisions, attribute values for the log call take precedence and override
//! [`Logger`] settings, but without modifying it.
//!
//! ## Cloning and Stacking
//!
//! [`Logger`]s can be cheaply cloned, extended and dropped. When a [`Logger`] is cloned, it inherits
//! all settings from the original, including levels, sinks (owned + inherited) and attributes.
//!
//! This allows for very flexible logging setups. New [`Logger`]s can just be extensions of
//! an original with extra arguments, have newly defined sinks and/or log levels - or both.
//!
//! In general, programs using Rasant will instantiate a single root logger via [`Logger::new()`],
//! and spawn nested clones as required.
//!
//! ## Asynchronous Operation
//!
//! By default, log operations lock until writes are propagated to all [Sink][`sink::Sink`]s associated
//! with a given [`Logger`].
//!
//! To improve performance when slow and/or a high number of [Sink][`sink::Sink`]s is
//! involved, Rasant supports dynamic asynchronous logging.
//!
//! Loggers can be switched to asynchronous mode via [`Logger::set_async`]. When enabled, log
//! operations defer writes by pushing them into a processing queue, and return immediately.
//!
//! Rasant will spawn a single thread to handle all asynchronous write operations, and
//! close it automatically once no async [`Logger`]s are present, and all their deferred
//! writes have been flushed.
//!
//! ## Error Handling
//!
//! For performance's sake, very few operations in Rasant's public API return errors, and
//! will [panic][`std::panic!`] upon failures instead.
//!
//! Pretty much all errors related to logging are unrecoverable anyway - these will either
//! happen at initialization time, or when trying to write to a [sink][`sink::Sink`].
//!
//! # License
//!
//! Rasant is distrubuted under the MIT license.
//!

#![deny(missing_docs)]
#![allow(dead_code)]

mod attributes;
mod console;
mod constant;
mod format;
mod level;
mod logger;
mod macros;
mod queue;
mod types;

// Public exported symbols
pub mod sink;
pub use attributes::value::{ToValue, Value};
pub use format::{FormatterConfig, OutputFormat};
pub use level::Level;
pub use logger::Logger;
