# Benchmark methodology

Run `cargo bench --locked --bench core > measurements.csv` in an idle environment.
The harness uses `std::hint::black_box` and optimized bench profile (thin LTO,
one codegen unit). Raw CSV rows represent one timed **batch**. Dividing elapsed
nanoseconds by batch operations gives mean time per named workload operation.

## Sampling

10 untimed warm-up batches, then 40 measured batches per case. The initial
record contains 3 separate process runs. Report the median and p95 of normalized
batch means, **not** the p95 of individual editor events. They are descriptive
statistics, not confidence intervals or hard real-time guarantees. Setup is
excluded where specified. Operations and size are explicit on every row.

## Cases

- `local_insert_delete`: insert then delete one ASCII byte in the middle; one
  operation is the **pair** of edits. 1 KiB, 64 KiB, 1 MiB and 8 MiB buffers.
- `scattered_insert_delete`: same pair at seeded spread-out positions; each pair
  restores the original text, so this is not accumulated fragmentation.
- `fragmented_insert`: 1,024 sustained scattered inserts into a 1 MiB starting
  buffer. Each measured batch starts fresh; setup is outside the timed region.
  One operation is one insert. This exposes the unbalanced piece-vector cost.
- `undo_redo`: one undo-plus-redo pair, with history enabled.
- `materialize`: export current chunks as one allocated contiguous String.
- `cursor_lookup_near_end`: byte-to-line/Unicode-scalar-column near the end of
  approximately 1 MiB of 11-byte lines; expected to expose linear cursor mapping.
- `open_warm_file`: open/validate approximately 1 MiB from a warmed filesystem cache.
- `save_staged_synced`: one-byte edit plus staged write, external-content checks,
  file sync and rename of 1 MiB. Disk/cache/provider behavior strongly affects this.
- `scan_1000_files`: discover 1,000 files in 10 directories under one project root.

## Baseline fairness

`std_string` is the Rust standard library `String`, **not another IDE**. Editing
comparisons disable history in Kedr; String also has no history. Both perform the
same ASCII operations and validate final size/content. String reserves enough
capacity before timed insert/delete cases; fragmented-insert setup reserves all
1,024 added bytes. Core added storage allocation remains part of the workload.
Order is fixed (Kedr then String), not randomized, so order/cache bias is possible.

## Limits

Measurements are from a shared Linux VM, not the maintainer's Arch/Hyprland/NVIDIA
machine. No UI latency, keystroke/render path, language-analysis speed, memory
benchmark, profiler or IDE comparison is included. A microbenchmark win does not
imply whole-editor superiority. Reproduce on real target machines and record
CPU, OS, compiler, build flags and source digest before making performance claims.

## Reproduce the report

The Rust harness needs no extra dependency. Optional capture/analysis helpers:

```bash
python scripts/benchmark.py --runs 3 --out results
# summarize.py requires pandas in the analysis environment
python scripts/summarize.py --raw results --out results
```

The summarizer checks row counts, case/sample uniqueness and positive timings;
independently rechecks every median with csv/statistics, and writes the evidence
object, full summary and result report. The initial source SHA-256 values in
`environment.json` must match the measured code, not a later revision.

## Version 0.2 paired comparison

The old vector engine and new AVL engine run the same expanded harness in three
separate processes each on the same VM. Added common cases: ASCII/Unicode buffer
construction, full TextBuffer cloning, Unicode cursor lookup and literal Unicode
search. Original cases are unchanged. This rerun is the comparison baseline;
it does not compare a new run against a favorable historical run on another machine.

`examples/footprint.rs` measures Linux whole-process VmHWM in KiB for independent
executables: a 4 MiB line-rich buffer, 4,096 sustained inserts, 1,024 history-enabled inserts, or 32 full buffer
clones. Three fresh processes per version/case. The high-water mark includes
construction/transient allocations and is not exclusive text heap or current RSS.
Only the core implementation changes; fixtures and workload semantics match.

New immutable Snapshot is lighter than a complete mutable TextBuffer clone, but
clone_buffer benchmarks the same existing clone API in both versions. No before/
after Snapshot benchmark is claimed because the old API had no equivalent method.
