# Indexed core performance comparison

Version 0.1 vector engine versus version 0.2 persistent AVL engine, with the
same expanded benchmark harness on the same shared Linux x86_64 VM.
Rust 1.99.0, Intel Xeon @ 2.50 GHz, two visible logical CPUs. Three processes
per engine: 29 cases, 120 measured batches/case, 6,960 timing rows total.
Ten warm-ups per case/run. No samples were discarded. All timing medians were
independently checked via csv/statistics. Values are normalized batch means,
not individual-event p95 or real IDE latency. See source fingerprints in results.

## Important matched workloads (approximately 1 MiB)

| Workload | Before, µs | After, µs | Before / after |
| --- | ---: | ---: | ---: |
| local_insert_delete | 0.2214 | 0.4997 | 0.44× |
| fragmented_insert | 18.9569 | 2.1555 | 8.79× |
| cursor_lookup_near_end | 2054.1629 | 0.2040 | 10069.43× |
| undo_redo | 0.1534 | 0.0396 | 3.88× |
| construct_unicode | 749.4797 | 2217.7542 | 0.34× |

The cursor/fragmentation/undo wins do not erase the local-edit and Unicode
construction regressions. Local pairs count insert plus delete; fragmented
operations count one insert. Unicode construction includes index building.
Future work should optimize tiny-edit paths and Unicode index construction
without removing snapshot/version safety or hiding retention costs.

## Whole-process memory

Independent Linux VmHWM measurements: 4 MiB line-rich text. Median of three
fresh processes/case/version, including setup and transient allocations.
This is peak process RSS, not current/exclusive heap, not MemoryStats.

| Fixture | Before peak RSS, MiB | After peak RSS, MiB |
| --- | ---: | ---: |
| lines | 5.867 | 7.402 |
| fragmented | 6.359 | 9.375 |
| history | 9.992 | 12.992 |
| clones | 134.117 | 7.500 |

Index/tree/history metadata increases the non-clone fixtures. Structural
sharing substantially reduces the 32-clone fixture; this does not promise
bounded memory when arbitrary long-lived snapshots or history are retained.

## Complete timing evidence

| Workload | Engine | Starting bytes | Before, µs | After, µs | Before / after |
| --- | --- | ---: | ---: | ---: | ---: |
| clone_buffer | piece_table | 1,048,576 | 60.4477 | 0.0618 | 977.92× |
| construct_ascii | piece_table | 1,048,576 | 754.1975 | 197.7051 | 3.81× |
| construct_unicode | piece_table | 1,048,572 | 749.4797 | 2217.7542 | 0.34× |
| cursor_lookup_near_end | piece_table | 1,048,575 | 2054.1629 | 0.2040 | 10069.43× |
| cursor_unicode_near_end | piece_table | 1,048,572 | 1221.3967 | 0.3964 | 3080.93× |
| fragmented_insert | piece_table | 1,048,576 | 18.9569 | 2.1555 | 8.79× |
| fragmented_insert | std_string | 1,048,576 | 9.9031 | 9.9216 | 1.00× |
| literal_search_unicode | piece_table | 1,048,572 | 2363.1500 | 2481.2591 | 0.95× |
| local_insert_delete | piece_table | 1,024 | 0.2265 | 0.4985 | 0.45× |
| local_insert_delete | piece_table | 65,536 | 0.2276 | 0.5154 | 0.44× |
| local_insert_delete | piece_table | 1,048,576 | 0.2214 | 0.4997 | 0.44× |
| local_insert_delete | piece_table | 8,388,608 | 0.2279 | 0.4947 | 0.46× |
| local_insert_delete | std_string | 1,024 | 0.0235 | 0.0236 | 1.00× |
| local_insert_delete | std_string | 65,536 | 0.6564 | 0.6904 | 0.95× |
| local_insert_delete | std_string | 1,048,576 | 18.0136 | 21.6374 | 0.83× |
| local_insert_delete | std_string | 8,388,608 | 293.3007 | 293.9658 | 1.00× |
| materialize | piece_table | 1,048,576 | 60.1115 | 61.3225 | 0.98× |
| open_warm_file | document | 1,048,576 | 886.3898 | 355.7532 | 2.49× |
| save_staged_synced | document | 1,048,576 | 1803.1040 | 1727.7095 | 1.04× |
| scan_1000_files | project | 0 | 602.6565 | 585.8965 | 1.03× |
| scattered_insert_delete | piece_table | 1,024 | 0.2267 | 0.6509 | 0.35× |
| scattered_insert_delete | piece_table | 65,536 | 0.2249 | 0.5046 | 0.45× |
| scattered_insert_delete | piece_table | 1,048,576 | 0.2211 | 0.7038 | 0.31× |
| scattered_insert_delete | piece_table | 8,388,608 | 0.2206 | 0.4930 | 0.45× |
| scattered_insert_delete | std_string | 1,024 | 0.0229 | 0.0231 | 0.99× |
| scattered_insert_delete | std_string | 65,536 | 0.7655 | 0.7956 | 0.96× |
| scattered_insert_delete | std_string | 1,048,576 | 18.9659 | 22.6894 | 0.84× |
| scattered_insert_delete | std_string | 8,388,608 | 318.4414 | 330.7258 | 0.96× |
| undo_redo | piece_table | 1,048,576 | 0.1534 | 0.0396 | 3.88× |

std_string rows are control workloads, not another IDE. Fixed engine/case
order, warm caches and shared-VM load can bias small differences; run-median
ranges and sample-p95 values are retained in comparison/timing.csv. These
are descriptive measurements, not confidence intervals. No causal claim is
made for unchanged project/OS code or small control fluctuations.

## Integration gains

Immutable snapshots, process-local document/revision tokens, stale/wrong-document
batch rejection, latest change envelopes, lazy search, allocation diagnostics
and an integration example. No UI, parsers, language servers or debugger were
added; these are future clients of the storage API.

Raw timing files: results/before and results/after. Raw RSS: results/memory.csv.
Source/environment fingerprints and reconciled evidence are checked in.
The original historical baseline remains in BENCHMARK_RESULTS.md.
