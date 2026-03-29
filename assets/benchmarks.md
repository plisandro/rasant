# Benchmarks 

## Integration test benchmarks

Basic benchmark tests, intended to gauge performance progress across versions, launched with
`cargo test --release --features=benchmark -- --show-output`.

All figures below were collected on 16-core AMD Ryzen 9 5950X system with 64GB of DDR4 memory.

### Version 0.3.0 (2026-03-30)

Remove all `String` generation from logging codepaths, optimize attribute maps.

```
--- Benchmark: single logger ---
[sync]
	wrote 1000000 compact log entries in 95.061682ms, average 95ns/op
	wrote 1000000 JSON log entries in 90.353409ms, average 90ns/op
	skipped 1000000 compact log entries in 2.174012ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.151221ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 834.480707ms, average 834ns/op
	wrote 1000000 JSON log entries in 812.074815ms, average 812ns/op
	skipped 1000000 compact log entries in 2.448651ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.836612ms, average 2ns/op

--- Benchmark: 50 nested loggers ---
[sync]
	wrote 1000000 compact log entries in 190.387245ms, average 190ns/op
	wrote 1000000 JSON log entries in 184.887471ms, average 184ns/op
	skipped 1000000 compact log entries in 2.529951ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.524952ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.244251002s, average 1.244µs/op
	wrote 1000000 JSON log entries in 1.231142195s, average 1.231µs/op
	skipped 1000000 compact log entries in 53.021519ms, average 53ns/op
	skipped 1000000 JSON log entries in 52.989749ms, average 52ns/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
	wrote 1000000 compact log entries in 351.532583ms, average 351ns/op
	wrote 1000000 JSON log entries in 309.24519ms, average 309ns/op
	skipped 1000000 compact log entries in 2.607461ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.746852ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.763556557s, average 1.763µs/op
	wrote 1000000 JSON log entries in 1.638452428s, average 1.638µs/op
	skipped 1000000 compact log entries in 53.294039ms, average 53ns/op
	skipped 1000000 JSON log entries in 53.30402ms, average 53ns/op

--- Benchmark: 50 multi-threaded nested loggers ---
[sync]
	wrote 1000000 compact log entries in 161.860448ms, average 161ns/op
	wrote 1000000 JSON log entries in 153.897705ms, average 153ns/op
	skipped 1000000 compact log entries in 1.377991ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.22438ms, average 1ns/op
[async]
	wrote 1000000 compact log entries in 1.037478419s, average 1.037µs/op
	wrote 1000000 JSON log entries in 1.040988431s, average 1.04µs/op
	skipped 1000000 compact log entries in 1.455561ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.503211ms, average 1ns/op
```

### Version 0.2.0 (2026-03-21)

Full async logging support.

```
--- Benchmark: single logger ---
[sync]
	wrote 1000000 compact log entries in 236.689932ms, average 236ns/op
	wrote 1000000 JSON log entries in 209.063272ms, average 209ns/op
	skipped 1000000 compact log entries in 2.544451ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.554001ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.063850668s, average 1.063µs/op
	wrote 1000000 JSON log entries in 1.061223427s, average 1.061µs/op
	skipped 1000000 compact log entries in 3.110891ms, average 3ns/op
	skipped 1000000 JSON log entries in 3.188111ms, average 3ns/op

--- Benchmark: 50 nested loggers ---
[sync]
	wrote 1000000 compact log entries in 372.262729ms, average 372ns/op
	wrote 1000000 JSON log entries in 353.267422ms, average 353ns/op
	skipped 1000000 compact log entries in 2.507881ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.525431ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.777245694s, average 1.777µs/op
	wrote 1000000 JSON log entries in 1.904141989s, average 1.904µs/op
	skipped 1000000 compact log entries in 53.387038ms, average 53ns/op
	skipped 1000000 JSON log entries in 52.987078ms, average 52ns/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
	wrote 1000000 compact log entries in 2.953245001s, average 2.953µs/op
	wrote 1000000 JSON log entries in 2.454856859s, average 2.454µs/op
	skipped 1000000 compact log entries in 2.484861ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.453261ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 8.790678239s, average 8.79µs/op
	wrote 1000000 JSON log entries in 12.053384927s, average 12.053µs/op
	skipped 1000000 compact log entries in 762.736713ms, average 762ns/op
	skipped 1000000 JSON log entries in 53.042009ms, average 53ns/op

--- Benchmark: 50 multi-threaded nested loggers ---
[sync]
	wrote 1000000 compact log entries in 822.507934ms, average 822ns/op
	wrote 1000000 JSON log entries in 783.084021ms, average 783ns/op
	skipped 1000000 compact log entries in 1.3622ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.248181ms, average 1ns/op
[async]
	wrote 1000000 compact log entries in 1.567337892s, average 1.567µs/op
	wrote 1000000 JSON log entries in 1.598383052s, average 1.598µs/op
	skipped 1000000 compact log entries in 1.52001ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.54654ms, average 1ns/op
```

### Version 0.1.0 (2026-03-15)

All log operations I/O perform streamed writes, with minimal string allocation.

```
--- Benchmark: single logger ---
[sync]
  wrote 1000000 compact log entries in 451.410456ms, average 451ns/op
  wrote 1000000 JSON log entries in 419.882445ms, average 419ns/op
  skipped 1000000 compact log entries in 418.184765ms, average 418ns/op
  skipped 1000000 JSON log entries in 389.581155ms, average 389ns/op

--- Benchmark: 50 nested loggers ---
[sync]
  wrote 1000000 compact log entries in 678.165075ms, average 678ns/op
  wrote 1000000 JSON log entries in 627.059056ms, average 627ns/op
  skipped 1000000 compact log entries in 544.775289ms, average 544ns/op
  skipped 1000000 JSON log entries in 416.698904ms, average 416ns/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
  wrote 1000000 compact log entries in 3.281495815s, average 3.281µs/op
  wrote 1000000 JSON log entries in 3.052457605s, average 3.052µs/op
  skipped 1000000 compact log entries in 3.055929586s, average 3.055µs/op
  skipped 1000000 JSON log entries in 2.999489397s, average 2.999µs/op
```

### Version 0.0.1 (2026-03-11)

Initial proof-of-concept implementation, without async support.

```
--- Benchmark: single logger ---
[sync]
  wrote 1000000 log entries in 803.284508ms, average 803ns/op

--- Benchmark: 50 nested loggers ---
[sync]
  wrote 1000000 log entries in 1.080452264s, average 1.08µs/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
  wrote 1000000 log entries in 4.999622168s, average 4.999µs/op
```
