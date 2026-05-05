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
//!   - [Leveled][`Level`], [structured](#attributes) contextual logging with [nanosecond precision](https://crates.io/crates/ntime).
//!   - [Simple API](#basic-logging), with support for [stacked logging](#stacking).
//!   - [Configurable log filters](#filtering).
//!   - Thread safe.
//!   - [Highly configurable log sinks](#configuring-sinks).
//!   - Text and JSON log output.
//!   - Support for [dynamic async logging](#asynchronous-logging) with constant lock time.
//!
//! # Examples
//!
//! ## Basic Logging
//!
//! [`Logger`]s can be easily initialized using sink defaults, and accessed via methods...
//!
//! ```rust
//! use rasant;
//! use rasant::Value;
//!
//! let mut log = rasant::Logger::new();
//! log.add_sink(rasant::sink::stderr::default()).set_level(rasant::Level::Info);
//!
//! log.set("program_name", "test");
//! log.info("hello world!");
//! log.warn_with("here's some context", [("line", Value::from(7))]);
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
//! r::set!(log, program_name = "test");
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
//! ## Attributes
//!
//! Rasant supports multiple attribute types (a.k.a [`Value`]s): single [`Scalar`] values,
//! lists and maps.
//!
//! ```rust
//! use rasant as r;
//!
//! let mut log = r::Logger::new();
//! log.add_sink(r::sink::stderr::default()).set_level(r::Level::Info);
//!
//! r::info!(log, "a single", value = 123.456);
//! let simple_list = [1, 2, 3, 4];
//! r::info!(log, "lists can be simple", list = r::list!(simple_list));
//! r::info!(log, "or have mixed types", list = r::list!("string!", 123.456, 789012 as usize));
//! r::info!(log, "and so can maps!", map = r::map!("key #1" => 123, 456 => 789.012));
//! ```
//!
//! ```text
//! 2026-05-04 03:58:41.189 +0200 [INF] a single value=123.456
//! 2026-05-04 03:58:41.189 +0200 [INF] lists can be simple list=[1, 2, 3, 4]
//! 2026-05-04 03:58:41.189 +0200 [INF] or have mixed types list=["string!", 123.456, 0xc0a14]
//! 2026-05-04 03:58:41.189 +0200 [INF] and so can maps! map={"key #1": 123, 456: 789.012}
//! ```
//!
//! ## Stacking
//!
//! All [`Logger`]s can be cheaply cloned, inheriting all settings from its
//! parents - including [Level][`level::Level`]s, [`sink`]s, [`filter`]s
//! and fixed [attributes](#attributes), allowing for very flexible setups.
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
//! 	r::set!(thread_log, thread_id = thread::current().id());
//!
//! 	r::info!(thread_log, "this will not log anything");
//! 	r::fatal!(thread_log, "but this will log to both stdout and stderr");
//! });
//! ```
//!
//! ## Configuring Sinks
//!
//! [`sink`]s can be configured to tweak multiple parameters, including time and
//! overall output format.
//!
//! ```rust
//! use rasant as r;
//!
//! let mut log = r::Logger::new();
//! log.set_level(r::Level::Info).add_sink(
//!     r::sink::stdout::new(r::sink::stdout::StdoutConfig {
//! 		formatter_cfg: r::FormatterConfig {
//! 			format: r::OutputFormat::Json,
//! 			time_format: r::TimeFormat::UtcNanosDateTime,
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
//! ## Filtering
//!
//! [`Logger`]s can apply configurable runtime [`filter`]s on log operations.
//! Supported filters include:
//!
//!   - [Multiple log levels][`filter::level::In`].
//!   - Log [message contents][`filter::matches::Message`].
//!   - Log [attributes presence][`filter::matches::AttributeKey`].
//!   - Log [attribute value contents][`filter::matches::AttributeValue`].
//!   - Several [log output sampling][`filter::sample`] criteria.
//!
//! Note that [`filter`]s are evaluated at logging time, and will
//! introduce (minimal) latency, regardless of [`Logger`]s having async mode
//! enabled.
//!
//! ```rust
//! use rasant as r;
//! use std::time::Duration;
//!
//! // Log a maximum of 10 Debug, Warning and Fatal updates per second, to keep SREs happy.
//! let mut log = r::Logger::new();
//! log
//!     .add_sink(r::sink::stdout::default())
//!     .set_all_levels()
//!     .add_filter(
//!         r::filter::level::In::new(
//!             r::filter::level::InConfig {
//!                 levels: [r::Level::Debug, r::Level::Warning, r::Level::Fatal],
//!             }))
//!     .add_filter(
//!         r::filter::sample::Burst::new(
//!             r::filter::sample::BurstConfig {
//!                 period: Duration::from_millis(1000),
//!                 max_updates: 10,
//!             }));
//!
//! r::info!(log, "this will not log");
//! r::debug!(log, "but");
//! r::fatal!(log, "these");
//! r::warn!(log, "will");
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
//! Rasant is built around individual [`Logger`] logging instances and [`sink`]s, which are
//! configurable destinations for log updates. When a log operation is performed, its level
//! is compared to the one defined for the [`Logger`] and, if applicable, the log is written
//! on all its [`sink`]s.
//!
//! Once a [`sink`] is added to a [`Logger`], it cannot be removed nor modified.
//!
//! ## Attributes
//!
//! ### Types
//!
//! Attributes are the defining quality of a structured logging system, expressed
//! as key-value pairs. In Rasant, keys are [`&str`], and [`Value`]s are a dedicated
//! type, supporting different configuration of [`Scalar`]s.
//!
//!   - [`Scalar`]s are the base unit for attribute values, mapping to basic data types:
//!     integers, floats and strings.
//!   - [`Value`] is a structured collection of [`Scalar`]s.
//!
//!  Rasant supports three basic [`Value`] types:
//!
//!   - [`Value::Scalar`] is a single [`Scalar`], and the most commonly used type.
//!   - [`Value::List`] is an ordered set of [`Scalar`]s.
//!   - [`Value::Map`] is an ordered set of key-value [`Scalar`]s.
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
//! ## Memory Management
//!
//! Rasant keeps all items associated with a [`Logger`] (keys, attribute values, their
//! [`Scalar`]s and all strings) in a group of owned vector arrays. No other heap
//! allocation is ever performed.
//!
//! These vectors will grow in size when needed - but never resize down. In practice,
//! this means that after just a few log calls vectors will grow to the size required
//! for normal operation, at which point all Rasant operations become effectively
//! zero-allocation.
//!
//! The vectored nature of [`Logger`] storage also makes cloning and dropping these
//! extremely efficient.
//!
//! ## Cloning and Stacking
//!
//! [`Logger`]s can be cheaply cloned, extended and dropped. When a [`Logger`] is cloned, it
//! inherits all settings from the original, including [level][`level::Level`]s, [`filter`]s,
//! [`sink`]s (owned + inherited) and attributes.
//!
//! This allows for very flexible logging setups. New [`Logger`]s can just be extensions of
//! an original with extra arguments, have newly defined sinks, log levels, filters and/or
//! async modes - or all of the above.
//!
//! In general, programs using Rasant will instantiate a single root logger via [`Logger::new()`],
//! and spawn nested clones as required.
//!
//! ## Asynchronous Operation
//!
//! By default, log operations lock until writes are propagated to all [`sink`]s associated
//! with a given [`Logger`].
//!
//! To improve performance when slow and/or a high number of [`sink`]s is involved, Rasant
//! supports dynamic asynchronous logging.
//!
//! Loggers can be switched to asynchronous mode via [`Logger::set_async`]. When enabled, log
//! operations defer writes by pushing them into a processing queue, and return immediately.
//!
//! Rasant will spawn a single thread to handle all asynchronous write operations, and
//! close it automatically once no async [`Logger`]s are present, and all their deferred
//! writes have been flushed.
//!
//! ## Log Filters
//!
//! [`Logger`]s supports optional, configurable [`filter`]s for log updates. These are evaluated,
//! in order, on every log operation, blocking [`sink`] writes unless the configured criteria for all
//! filters is met. Multiple filters can be stacked to combine their behavior.
//!
//! [`filter`]s are evaluated after normal level checks, so their output is affected by each
//! [`Logger`]s log level. When using level-based filters (f.ex. [Levels][`crate::filter::level::In`]),
//! consider enabling [set_all_levels()][`crate::logger::Logger::set_all_levels`] to avoid
//! unexpected interactions.
//!
//! Note that [`filter`]s are evaluated at logging time, even for [`Logger`]s in
//! asynchronous mode; as a result, every [`filter`] will introduce additional latency
//! on **all** log operations for that [`Logger`].
//!
//! ## Error Handling
//!
//! For performance's sake, very few operations in Rasant's public API return errors, and
//! will [panic][`std::panic!`] upon failures instead.
//!
//! Pretty much all errors related to logging are unrecoverable anyway - these will either
//! happen at initialization time, or when trying to write to a [`sink`].
//!
//! # Repository
//!
//! This project is currently hosted at [GitHub](https://github.com/plisandro/rasant).
//!
//! # License
//!
//! Rasant is distrubuted under the MIT license.

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
pub mod filter;
pub mod sink;
pub use attributes::scalar::Scalar;
pub use attributes::value::Value;
pub use format::{FormatterConfig, OutputFormat};
pub use level::Level;
pub use logger::Logger;
/// [`ntime::Format`], re-exported for convenience.
pub use ntime::Format as TimeFormat;
pub use types::{AttributeString, AttributeStringSeek};
