# Changelog

A list of important changes for relevant `rasant` releases.

## Version 0.7.0 (2026-05-05)

New attribute engine, with zero-alloc handling of all attribute types - including long strings.
Added support for list and maps as attribute values.
Added support for configurable log filters.

## Version 0.6.0 (2026-04-16)

Zero-alloc handling of all attribute types, with the exception of long `String`s. Switched
benchmarking over to [Divan](https://crates.io/crates/divan).

## Version 0.5.0 (2026-04-07)

Optimize handling of async log operations by removing unnecessary R/W's on common mutexed
items.

These are now _significantly_ faster - particularly on multi-threaded applications.

## Version 0.4.0 (2026-04-04)

Initial public release. Optimized attributes maps making most operations zero allocation.

## Version 0.3.0 (2026-03-30)

Removed all `String` generation from logging codepaths, optimize attribute maps.

## Version 0.2.0 (2026-03-21)

Full async logging support.

## Version 0.1.0 (2026-03-15)

First working prototype, with all log operations I/O perform streamed writes and minimal string allocation.
