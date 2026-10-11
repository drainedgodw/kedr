# Initial benchmark results

Three release-process runs on a shared Linux x86_64 VM; 2,880 timed batches,
24 cases, 120 measured batches per case. 10 warm-ups per case/run. Rust 1.99.0.
Intel Xeon @ 2.50 GHz, 2 visible logical CPUs. This is not an Arch desktop test.

Times below are median microseconds per **named workload operation**, computed
from normalized batch means. Sample-p95 is not individual-event p95. No samples
were excluded. All medians independently reconciled via Python csv/statistics.

## Editing against the standard library

Both engines have history disabled. Insert/delete cases count one pair; the
fragmented case counts one sustained insert. Do not compare those rows as identical operations.

| Workload | Starting bytes | Kedr, µs | String, µs | String / Kedr |
| --- | ---: | ---: | ---: | ---: |
| local_insert_delete | 1,024 | 0.221 | 0.024 | 0.11× |
| local_insert_delete | 65,536 | 0.222 | 0.678 | 3.05× |
| local_insert_delete | 1,048,576 | 0.225 | 18.160 | 80.73× |
| local_insert_delete | 8,388,608 | 0.222 | 294.737 | 1328.42× |
| scattered_insert_delete | 1,024 | 0.228 | 0.023 | 0.10× |
| scattered_insert_delete | 65,536 | 0.224 | 0.781 | 3.49× |
| scattered_insert_delete | 1,048,576 | 0.225 | 18.959 | 84.40× |
| scattered_insert_delete | 8,388,608 | 0.229 | 319.620 | 1395.82× |
| fragmented_insert | 1,048,576 | 19.018 | 10.044 | 0.53× |

Kedr loses the small restored-pair cases and the fragmented-insert case.
Its local-pair advantage at large sizes is a microbenchmark result, not a
comparison against PyCharm or a production editor. Piece-vector fragmentation
must be addressed before treating this engine as ready for interactive workloads.

## Other core workloads

| Workload | Median, µs | Sample-p95, µs |
| --- | ---: | ---: |
| cursor_lookup_near_end | 2055.432 | 2331.879 |
| materialize | 58.615 | 62.908 |
| open_warm_file | 894.170 | 936.233 |
| save_staged_synced | 1583.741 | 2379.415 |
| scan_1000_files | 624.057 | 688.593 |
| undo_redo | 0.152 | 0.195 |

## Next measured priorities

1. Balanced piece index: improve sustained scattered inserts, not just easy local edits.
2. Line/cursor index: avoid rescanning nearly the whole document on cursor queries.
3. Streaming search and scheduled added-storage compaction.

Filesystem cache/order bias and shared VM scheduling limit generalization. No
peak-memory, UI, language-analysis, IDE-comparison or power-loss benchmark was run.

Raw CSV files, source fingerprints and environment metadata are in `results/`.
See `docs/BENCHMARKS.md` for exact workloads and setup exclusions.
