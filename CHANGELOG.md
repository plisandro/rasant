# Changelog

A list of important changes for relevant `rasant` releases.

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
