# Changelog

A list of important changes for relevant `rasant` releases.

## Version 1.3.0 (2026-11-11)

High-performance async queue implementation, improved third-party APIs, bugfixes.

  - New custom, low contention double-buffered global async queue handler. This not only
    minimizes memory fragmentation by reusing heap whenever possible, but also performs
    _much_ better than the previous version, particularly regarding throughput and contention.
  - All heap data structures now get a (small) default capacity, minimizing reallocs for
    small items.
  - Improved API for third-party sinks.
  - Formalize support for third-party `Scalar` casting on custom types.
  - Add `Scalar` casting support for `char`, `&Option<>`, `&String` and all `net::*` types.
  - Fix a bug where `Trace` and `Panic` log updates could incorrectly modify fixed
    attributes for a `Logger`.

## Version 1.2.0 (2026-08-23)

Bugfixes and minor feature updates.

  - Drop global logger IDs, as these imposed a hard limit on 2^32 `Logger` instances being
    created/cloned within a process.
  - `Panic` level updates include caller filename and line as attributes.
  - Maximum logger depth is now 4096.
  - Add `None` as a new `Scalar` type, and casting from `Optional<>` on all supported `From` types.
  - Turn `rasant::logger::Logger::panic()` and `rasant::logger::Logger::panic_with()` into
    `never`s, so these can be used as a replacement for `panic!()`.
  - Make `rasant::sink::PartialLogUpdate` non-public so third-party sinks don't attempt to
    access implementation details.
  - Improve documentation, clean up diagrams.
  - Customizable optional filename prefixes on `LogFile` sinks.
  - `Scalar` casting support for `std::ffi::OsString`, `std::ffi::OsStr`, `std::path::Path` and `std::path::PathBuf`.

## Version 1.1.0 (2026-07-22)

Bugfixes and minor feature updates.

  - `Format::Full` and `Format::ColorFull` I/O sink formats are now 100% buffer-less.
  - Fix spacing ellipsizing bug for `Format::Full` and `Format::ColorFull`.
  - `Trace` level updates include caller filename and line as attributes.
  - Support initializing `List` and `Map` values from `Vec`tors.
  - Rework log macros to accept `Logger`s as expressions instead of items.
  - `Scalar` casting support for `std::net` sockets/addresses.
  - Support resolving I/O output formats by name.

## Version 1.0.0 (2026-06-22)

  - Improved overall performance.
  - Support for third-party sinks and filters.
  - Log attribute metadata for enhanced sink output.
  - Full log output with stacking hierarchy.

## Version 0.8.0 (2026-05-18)

  - [CBOR](https://cbor.io/) output formatting.
  - Journald sink.
  - Syslog [RFC 3164](https://datatracker.ietf.org/doc/html/rfc3164) and [RFC 5424](https://datatracker.ietf.org/doc/html/rfc5424) sink.

## Version 0.7.0 (2026-05-05)

  - New attribute engine, with zero-alloc handling of all attribute types - including long strings.
  - Added support for list and maps as attribute values.
  - Added support for configurable log filters.

## Version 0.6.0 (2026-04-16)

  - Zero-alloc handling of all attribute types, with the exception of long `String`s.
  - Switched benchmarking over to [Divan](https://crates.io/crates/divan).

## Version 0.5.0 (2026-04-07)

  - Optimize handling of async log operations by removing unnecessary R/W's on common mutexed items.
    These are now _significantly_ faster - particularly on multi-threaded applications.

## Version 0.4.0 (2026-04-04)

  - Initial public release.
  - Optimized attributes maps making most operations zero allocation.

## Version 0.3.0 (2026-03-30)

  - Removed all `String` generation from logging codepaths, optimize attribute maps.

## Version 0.2.0 (2026-03-21)

  - Full async logging support.

## Version 0.1.0 (2026-03-15)

  - First working prototype, with all log operations I/O perform streamed writes and minimal string allocation.
