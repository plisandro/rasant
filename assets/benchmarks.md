# Benchmarks 

## Integration test benchmarks

Basic benchmark tests, intended to gauge performance progress across versions, launched with
`cargo test --release --features=benchmark -- --show-output`.

All figures below were collected on 16-core AMD Ryzen 9 5950X system with 64GB of DDR4 memory.

### Version 0.5.0 (2026-04-07)

Optimize handling of async log operations.

```
--- Benchmark: single logger ---
[sync]
	wrote 1000000 compact log entries in 66.124354ms, average 66ns/op
	wrote 1000000 JSON log entries in 65.969635ms, average 65ns/op
	skipped 1000000 compact log entries in 2.563116ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.554296ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 837.521392ms, average 837ns/op
	wrote 1000000 JSON log entries in 824.535125ms, average 824ns/op
	skipped 1000000 compact log entries in 3.018033ms, average 3ns/op
	skipped 1000000 JSON log entries in 3.083163ms, average 3ns/op

--- Benchmark: 50 nested loggers ---
[sync]
	wrote 1000000 compact log entries in 70.662978ms, average 70ns/op
	wrote 1000000 JSON log entries in 69.322467ms, average 69ns/op
	skipped 1000000 compact log entries in 2.401866ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.413617ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.298915357s, average 1.298µs/op
	wrote 1000000 JSON log entries in 1.313931904s, average 1.313µs/op
	skipped 1000000 compact log entries in 2.896094ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.914194ms, average 2ns/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
	wrote 1000000 compact log entries in 202.135191ms, average 202ns/op
	wrote 1000000 JSON log entries in 200.012082ms, average 200ns/op
	skipped 1000000 compact log entries in 2.390117ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.374517ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.956121288s, average 1.956µs/op
	wrote 1000000 JSON log entries in 1.725901453s, average 1.725µs/op
	skipped 1000000 compact log entries in 2.913994ms, average 2ns/op
	skipped 1000000 JSON log entries in 3.106722ms, average 3ns/op

--- Benchmark: 50 multi-threaded nested loggers ---
[sync]
	wrote 1000000 compact log entries in 183.235505ms, average 183ns/op
	wrote 1000000 JSON log entries in 181.405756ms, average 181ns/op
	skipped 1000000 compact log entries in 1.252083ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.238223ms, average 1ns/op
[async]
	wrote 1000000 compact log entries in 192.024197ms, average 192ns/op
	wrote 1000000 JSON log entries in 91.088975ms, average 91ns/op
	skipped 1000000 compact log entries in 1.624381ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.428192ms, average 1ns/op
```

### Version 0.4.0 (2026-04-04)

Reworked attribute maps making most operations zero allocation, minor optimizations.

```
--- Benchmark: single logger ---
[sync]
	wrote 1000000 compact log entries in 68.433478ms, average 68ns/op
	wrote 1000000 JSON log entries in 68.940067ms, average 68ns/op
	skipped 1000000 compact log entries in 2.283092ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.071971ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.068090936s, average 1.068µs/op
	wrote 1000000 JSON log entries in 859.384891ms, average 859ns/op
	skipped 1000000 compact log entries in 2.667301ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.699961ms, average 2ns/op

--- Benchmark: 50 nested loggers ---
[sync]
	wrote 1000000 compact log entries in 70.754138ms, average 70ns/op
	wrote 1000000 JSON log entries in 71.062479ms, average 71ns/op
	skipped 1000000 compact log entries in 2.383622ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.344671ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 1.530982379s, average 1.53µs/op
	wrote 1000000 JSON log entries in 1.395126025s, average 1.395µs/op
	skipped 1000000 compact log entries in 52.900729ms, average 52ns/op
	skipped 1000000 JSON log entries in 52.895409ms, average 52ns/op

--- Benchmark: 50 nested loggers with increasing arguments ---
[sync]
	wrote 1000000 compact log entries in 255.630361ms, average 255ns/op
	wrote 1000000 JSON log entries in 207.853134ms, average 207ns/op
	skipped 1000000 compact log entries in 2.357061ms, average 2ns/op
	skipped 1000000 JSON log entries in 2.346811ms, average 2ns/op
[async]
	wrote 1000000 compact log entries in 2.004260249s, average 2.004µs/op
	wrote 1000000 JSON log entries in 2.046578553s, average 2.046µs/op
	skipped 1000000 compact log entries in 102.952706ms, average 102ns/op
	skipped 1000000 JSON log entries in 52.896649ms, average 52ns/op

--- Benchmark: 50 multi-threaded nested loggers ---
[sync]
	wrote 1000000 compact log entries in 184.994561ms, average 184ns/op
	wrote 1000000 JSON log entries in 183.519921ms, average 183ns/op
	skipped 1000000 compact log entries in 1.272281ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.334721ms, average 1ns/op
[async]
	wrote 1000000 compact log entries in 1.694335899s, average 1.694µs/op
	wrote 1000000 JSON log entries in 1.724104226s, average 1.724µs/op
	skipped 1000000 compact log entries in 1.588431ms, average 1ns/op
	skipped 1000000 JSON log entries in 1.420131ms, average 1ns/op
```

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
