# Core 0.3: extensibility and endurance

This iteration adds safe integration primitives, not an IDE or product catalog.
There is no evidence here that Kedr outperforms JetBrains.

## Additions

- UTF-16 code-unit coordinates alongside the existing Unicode-scalar API. Byte
  editing stays UTF-8; surrogate halves are rejected. This is not an LSP server.
- Optional bounded invalidation history; disabled by default. Consumers receive
  explicit gap/wrong-document/future-version errors, not silently incomplete events.
- Nonwrapping content revisions; typed undo/redo failures preserve state.
- Version-guarded prepared compaction: rebuild a Snapshot on a worker, adopt only
  for that exact current document/revision. No automatic worker scheduler.
- Endurance executable and standard-library Python collector with source hashes,
  individual-operation samples, bounded reader checks and memory checkpoints.

47 tests pass in debug and optimized release locally on Linux x86_64 / Rust 1.99.0.
Formatting, Clippy and Rustdoc with denied warnings also pass. The collector also
passed 10,000-operation smoke runs. Stable-platform CI is configured for Linux,
Windows and macOS plus separate Rust 1.88 checks; configuration is not success.

## Final-code endurance

One deterministic process run per case on the same shared Linux VM. CPU/OS/compiler
and exact source digests are in `results/hardening/endurance/environment.json`.
Normal release profile, no explicit LTO. History is capped at 128 steps / 8 MiB
logical bytes; the invalidation journal at 256 entries. Two retained checkpoint
snapshots and a bounded two-message background channel prevent unbounded test
retention. No warmup samples are discarded.

ASCII starts at 16 MiB and flips seeded individual ASCII bytes. Unicode starts at
30,720 bytes and replaces seeded complete scalars with variable-length characters.
They are different workloads/sizes, **not an ASCII-versus-Unicode speed ranking**.
The separate String reference is updated outside the timer. Final text/revision
and 16 complete-text/UTF-16/position checkpoints per case pass; 245 background
snapshot content/UTF-16 checks per case pass, with old snapshots verified as well.
The final Unicode buffer is 29,711 bytes.

Each sample measures one real replacement, including synchronous compaction if
pre-edit pieces exceed 65,536. Reference work, checks, reader submissions, memory
sampling and output are excluded. Concurrent reader scheduling can still affect
latency. Quantiles use linear interpolation over individual elapsed wall times.

| Fixture | Operations | p50 µs | p95 µs | p99 µs | Maximum ms | Operations >1 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ascii | 1,000,000 | 9.421 | 17.416 | 23.277 | 62.034 | 115 |
| unicode | 1,000,000 | 7.686 | 10.385 | 18.427 | 4.942 | 12 |

ASCII performed 30 synchronous compactions, ending with 31,908 pieces / height 19.
Unicode performed no compaction, ending with 17,920 pieces / height 18. The large
maximum must not be hidden behind p99: synchronous rebuild/reclamation can visibly
stall an interactive controller. Background preparation is available, but its
installation/reclamation still has no hard real-time guarantee. No separate
background-compaction latency claim is made.

## Memory evidence

Linux VmHWM is whole-process peak RSS, **not the core's exclusive heap**. It includes
reference copies, old/queued snapshots, the 8 MB timing vector, background stack,
allocator and transient compaction allocations. memory_stats traverses reachable
buffer/history allocations but excludes external snapshot roots and allocator
costs. The sampled maximum below is not a peak-allocation guarantee.

| Fixture | Process peak RSS MiB | Max sampled reachable MiB | Final sampled reachable MiB |
| --- | ---: | ---: | ---: |
| ascii | 234.21 | 31.31 | 23.62 |
| unicode | 28.89 | 5.58 | 5.52 |

All 32 memory checkpoints are retained in `results/hardening/endurance/`. Reachable
bytes stabilize or fluctuate within the observed range in these finite fixtures; process RSS is affected by
retention, allocator behavior and progressively touched sample storage. This does
not establish absence of leaks or a physical memory cap for arbitrary workloads.

## Matched microcomparison: 0.2 → final 0.3

Unchanged 29-case harness; 3 process runs per version, 40 measured batches/case/run
following 10 warmup batches. 6,960 raw batch rows. These medians normalize batch
elapsed time by the named workload's operation count; **they are not individual
event quantiles**. Both versions run in the same VM, but fixed order, shared-host
scheduling and run-to-run variation limit causal interpretation.

| Kernel workload | Bytes | 0.2 median µs/op | 0.3 median µs/op | 0.3 / 0.2 |
| --- | ---: | ---: | ---: | ---: |
| `clone_buffer` | 1,048,576 | 0.0622 | 0.0623 | 1.002 |
| `construct_ascii` | 1,048,576 | 196.7537 | 195.3349 | 0.993 |
| `construct_unicode` | 1,048,572 | 2239.6489 | 2181.1975 | 0.974 |
| `cursor_lookup_near_end` | 1,048,575 | 0.2043 | 0.2029 | 0.993 |
| `cursor_unicode_near_end` | 1,048,572 | 0.3984 | 0.4012 | 1.007 |
| `fragmented_insert` | 1,048,576 | 2.1378 | 2.1615 | 1.011 |
| `literal_search_unicode` | 1,048,572 | 2508.8911 | 2462.0889 | 0.981 |
| `local_insert_delete` | 1,024 | 0.4974 | 0.5186 | 1.043 |
| `local_insert_delete` | 65,536 | 0.4949 | 0.5180 | 1.047 |
| `local_insert_delete` | 1,048,576 | 0.4948 | 0.5169 | 1.044 |
| `local_insert_delete` | 8,388,608 | 0.4947 | 0.5172 | 1.045 |
| `materialize` | 1,048,576 | 63.3167 | 58.4643 | 0.923 |
| `scattered_insert_delete` | 1,024 | 0.4994 | 0.7588 | 1.519 |
| `scattered_insert_delete` | 65,536 | 0.5012 | 0.6471 | 1.291 |
| `scattered_insert_delete` | 1,048,576 | 0.7720 | 0.5172 | 0.670 |
| `scattered_insert_delete` | 8,388,608 | 0.5022 | 0.5161 | 1.028 |
| `undo_redo` | 1,048,576 | 0.0397 | 0.0445 | 1.123 |

No general speedup is claimed. Local insert/delete pairs cost roughly 4–5% more;
undo/redo pairs roughly 12% more in these runs. Pooled small scattered-edit results
are worse (about 52% at 1 KiB and 29% at 64 KiB), with substantial run variation:
0.3 per-run 1 KiB medians range about 0.518–0.832 µs. The apparent 1 MiB scattered
speedup is also variable in the baseline and is not treated as an architectural
win. Full per-case evidence, including String controls, is committed rather than
cherry-picked. Replicate on real target hardware before assigning causes.

An intermediate implementation redundantly computed UTF-16 prefix ranks for
ordinary scalar cursors. That measured near-end Unicode lookup at about 0.748 µs,
versus 0.398 µs in 0.2. Encoding-specific prefix queries remove this regression:
the final result is about 0.401 µs. Intermediate data was retained during analysis;
all published final timings match the final source digests.

## Reproduction and audit

```bash
cargo test --locked
cargo test --locked --release
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
python3 scripts/endurance.py --operations 1000000 --out results/local-endurance
python3 scripts/benchmark.py --runs 3 --out results/local-micro
```

Raw micro CSVs, both source/environment fingerprints, reconciled summaries and
memory checkpoints are in `results/hardening/`. The two million individual timing
rows are delivered separately in `kedr-0.3-evidence.zip` to avoid bloating the source
repository. `endurance-summary.json` records their SHA-256 hashes. The archive also
contains provenance, comparison CSVs, test logs and the analysis scripts.

The datasets were profiled for row counts, nulls, duplicate rows and identifiers.
Every microcase median was independently reconciled with csv/statistics. All four
endurance quantiles were independently rechecked with a separate sorted stdlib
parse against NumPy's interpolation; the collector's own summary also agrees.
`checks.json` records 2,000,000 individual rows and 6,960 micro rows.

This is deterministic endurance, not long-running fuzzing, UI/LSP/debugger testing,
a JetBrains benchmark, a universal memory proof or a real-time guarantee. The next
performance target is controlled compaction/reclamation scheduling, plus repeatable
latency/memory runs on actual Arch Linux and Windows hardware.
