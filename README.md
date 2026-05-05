# Rasant

<p>
    <picture>
      <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/plisandro/rasant/master/assets/rasant_title_light_horizontal.png" width="350px">
      <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/plisandro/rasant/master/assets/rasant_title_dark_horizontal.png" width="350px">
      <img src="https://raw.githubusercontent.com/plisandro/rasant/master/assets/rasant_title_light_horizontal.png" width="350px" />
    </picture>
    <br>
</p>

[![](https://img.shields.io/crates/v/rasant.svg)][crates-io]
[![](https://docs.rs/rasant/badge.svg)][api-docs]

Rasant is a lightweight, high performance and flexible Rust library for structured logging,
inspired by the likes of [zap](https://github.com/uber-go/zap) and [zerolog](https://github.com/rs/zerolog).

It offers [nanosecond precision](https://github.com/plisandro/ntime), stackable logging and
[outstanding performance](assets/benchmarks.md): on modern systems, Rasant can process and
dispatch logs to multiple sinks in tens of nanoseconds, being normally bottlenecked by I/O
operations. Can't wait that long? There's built-in [async support](#asynchronous-logging)!

![Sample text output image](assets/sample_output_text.png)

![Sample JSON output image](assets/sample_output_json.png)

## Main Features

  - Minimal dependencies.
  - [Blazing fast](assets/benchmarks.md) performance, with zero allocations on most operations.
  - Leveled, [structured](#attributes) contextual logging with [nanosecond precision](https://github.com/plisandro/ntime).
  - [Simple API](#basic-examples), with support for [stacked logging](#stacking).
  - [Configurable log filters](#filtering).
  - Thread safe.
  - [Highly configurable log sinks](#configuring-sinks).
  - Text and JSON log output.
  - Support for [dynamic async logging](#asynchronous-logging) with constant lock time.

See also [Why Rasant?](assets/why_rasant.md) for more background, and comparsions with other
logging solutions for Rust.

## Usage 

Latest stable release is **v0.7.0**. To use it, add the `rasant` crate to your `Cargo.toml` file:

```toml
[dependencies]
rasant = "0.7.0"
```

Rasant is under active development and on track for a v1.0.0 release. You may see small public
API changes until then, but the library is otherwise stable and fully functional.

## Getting Started

### Basic Examples

Loggers can be easily initialized using sink defaults, and accessed via methods...

```rust
use rasant;
use rasant::Value;

let mut log = rasant::Logger::new();
log.add_sink(rasant::sink::stderr::default()).set_level(rasant::Level::Info);

log.set("program_name", "test");
log.info("hello world!");
log.warn_with("here's some context", [("line", Value::from(7))]);
log.debug("and i'm ignored :(");
```

...or the _much_ nicer macro API:

```rust
use rasant as r;

let mut log = r::Logger::new();
log.add_sink(r::sink::stderr::default()).set_level(r::Level::Info);

r::set!(log, program_name="test");
r::info!(log, "hello world!");
r::warn!(log, "here's some context", line = 7);
r::debug!(log, "and i'm ignored :(");
```

```
2026-04-03 17:16:03.773 +0200 [INF] hello world! program_name="test"
2026-04-03 17:16:03.773 +0200 [WRN] here's some context program_name="test" line=7
```

### Attributes

Rasant supports multiple attribute types: single scalars, lists and maps.

```rust
use rasant as r;

let mut log = r::Logger::new();
log.add_sink(r::sink::stderr::default()).set_level(r::Level::Info);

r::info!(log, "a single", value = 123.456);
let simple_list = [1, 2, 3, 4];
r::info!(log, "lists can be simple", list = r::list!(simple_list));
r::info!(log, "or have mixed types", list = r::list!("string!", 123.456, 789012 as usize));
r::info!(log, "and so can maps!", map = r::map!("key #1" => 123, 456 => 789.012));
```

```
2026-05-04 03:58:41.189 +0200 [INF] a single value=123.456
2026-05-04 03:58:41.189 +0200 [INF] lists can be simple list=[1, 2, 3, 4]
2026-05-04 03:58:41.189 +0200 [INF] or have mixed types list=["string!", 123.456, 0xc0a14]
2026-05-04 03:58:41.189 +0200 [INF] and so can maps! map={"key #1": 123, 456: 789.012}
```

### Stacking

All loggers can be cheaply cloned, inheriting all settings from its parents - including
levels, sinks, filters and fixed attributes - allowing for very flexible setups. For example,
to have all errors (or higher) within a thread logged to `stderr`:

```rust
use rasant as r;
use std::thread;

let mut log = r::Logger::new();
log.add_sink(r::sink::stdout::default()).set_level(r::Level::Info);
r::info!(log, "main logs to stdout only");

let mut thread_log = log.clone();
thread::spawn(move || {
	thread_log.add_sink(r::sink::stderr::default()).set_level(r::Level::Error);
	r::set!(thread_log, thread_id = thread::current().id());

	r::info!(thread_log, "this will not log anything");
	r::fatal!(thread_log, "but this will log to both stdout and stderr");
});
```

### Configuring Sinks

Sinks can be configured to tweak multiple parameters, including time and
overall output format.

```rust
use rasant as r;

let mut log = r::Logger::new();
log.set_level(r::Level::Info).add_sink(
    r::sink::stdout::new(r::sink::stdout::StdoutConfig {
		formatter_cfg: r::sink::format::FormatterConfig {
			format: r::sink::format::OutputFormat::Json,
			time_format: r::TimeFormat::UtcNanosDateTime,
			..r::sink::format::FormatterConfig::default()
		},
		..r::sink::stdout::StdoutConfig::default()
	})
);

r::info!(log, "hello!");
```

```
{"time":"2026-04-03 16:03:04.481888522","level":"info","message":"hello!"}
```

### Asynchronous Logging

All loggers can dynamically enable/disable async writes.

When in async mode, log operations have a slightly longer (as details are
copied into a queue) _but fixed_ lock time, making it ideal f.ex. for
logging into slow storage without compromising overall performance.

```rust
use rasant as r;

let mut log = r::Logger::new();
log.set_level(r::Level::Info).add_sink(r::sink::stdout::default());

r::info!(log, "this is writen synchronously");
log.set_async(true);
r::info!(log, "and these write");
r::warn!(log, "asynchronously, but");
r::info!(log, "in order!");
```

### Filtering

Rasant supports optional, configurable runtime filters for all log operations,
including filtering by levels, log message, attribute key/value contents,
and multiple sampling filters for statistical and monitoring purposes.

```rust
use rasant as r;
use std::time::Duration;

// Log a maximum of 10 Debug, Warning and Fatal updates per second, to keep SREs happy.
let mut log = r::Logger::new();
log
    .add_sink(r::sink::stdout::default())
    .set_all_levels()
    .add_filter(
        r::filter::level::In::new(
            r::filter::level::InConfig {
                levels: [r::Level::Debug, r::Level::Warning, r::Level::Fatal],
            }))
    .add_filter(
        r::filter::sample::Burst::new(
            r::filter::sample::BurstConfig {
                period: Duration::from_millis(1000),
                max_updates: 10,
            }));

r::info!(log, "this will not log");
r::debug!(log, "but");
r::fatal!(log, "these");
r::warn!(log, "will");
```

## Documentation

  * [API documentation][api-docs]
  * [CHANGELOG]
  * [Real-world benchmarks][benchmarks]

## Support

Comments, feedback and bug reports are always welcome!

You can reach out through regular GitHub issues and bug reports, or via the
[rasant gitter channel](https://app.gitter.im/#/room/#rasant:gitter.im).

Contributions will be accepted under the [project's license](#licence).

## To-Do's

Rasant is under active development, with more features planned for future versions.

  - New output formants (hierarchical pretty print?)
  - New sink types (f.ex. [syslog](https://en.wikipedia.org/wiki/Syslog))
  - Support for third-party log sinks
  - Binary output formats, such as [CBOR](https://cbor.io/) and [protobuf](https://protobuf.dev/).

## License

Rasant is distrubuted under the [MIT license][mit].

<img src="assets/Developed-By-a-Human-Not-By-AI-Badge-white.svg" title="Courtesy of https://notbyai.fyi/" height="80px"/>

[api-docs]: https://docs.rs/rasant
[crates-io]: https://crates.io/crates/rasant
[CHANGELOG]: CHANGELOG.md
[benchmarks]: assets/benchmarks.md
[mit]: LICENSE
