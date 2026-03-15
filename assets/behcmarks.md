# Benchmarks 

## Integration test benchmarks

Basic benchmark tests, intended to gauge performance progress across versions, launched with `cargo test --show-output`.
Note that these benchmarks build with `test` profile, and will not represent actual raw performance for release builds.

All figures below were collected on 16-cre AMD Ryzen 9 5950X system with 64GB of DDR4 memory.

### Latest: 2026-03-15, version 0.1.0

All log operations I/O perform streamed writes, with zero string allocation.

```
---- benchmarks::black_hole_single stdout ----
wrote 1000000 log entries in 1.770373192s, average 1.77µs/op

---- benchmarks::black_hole_nested stdout ----
wrote 1000000 log entries in 2.489003161s via 50 logger instances, average 2.489µs/op

---- benchmarks::black_hole_nested_with_arguments stdout ----
wrote 1000000 log entries in 12.309710636s via 50 logger instances with up to 50 arguments, average 12.309µs/op
```

### Version 0.0.1

Initial proof-of-concept implementation, without async support.

```
---- benchmark_tests::black_hole_single stdout ----
wrote 1000000 log entries in 2.399970456s, average 2.399µs/op

---- benchmark_tests::black_hole_nested stdout ----
wrote 1000000 log entries in 3.269218658s via 50 logger instances, average 3.269µs/op

---- benchmark_tests::black_hole_nested_with_arguments stdout ----
wrote 1000000 log entries in 16.446279427s via 50 logger instances with up to 50 arguments, average 16.446µs/op
```
