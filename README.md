# Kedr

A GUI-independent editor core in Rust. **Core first. No IDE products or UI.**
No runtime dependencies, mandatory accounts, telemetry or cloud services.

## Current scope

- Piece-table UTF-8 text storage with streamed chunks and explicit materialization.
- Validated byte-range edits; atomic non-overlapping edit batches.
- Bounded delta-based undo/redo and explicit storage compaction.
- Line counts, Unicode-scalar cursor mapping and literal search.
- UTF-8 document loading, dirty tracking and staged/synced saves.
- External-change refusal and bounded project discovery without symlink traversal.
- Deterministic reference-model tests and reproducible release benchmarks.

This is an early library, **not** an IDE engine with language intelligence. There
is no parser, semantic index, debugger, plugin system, GUI, installer or product
catalog. No superiority over JetBrains or other editors is claimed.

## Build and test

Rust 1.88+:

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo bench --locked --bench core > measurements.csv
```

See [architecture](docs/ARCHITECTURE.md), [benchmark methodology](docs/BENCHMARKS.md),
[validation](docs/VALIDATION.md) [measured baseline](BENCHMARK_RESULTS.md) and the checked-in results in `results/`.
`measurements.csv` contains raw timed batches, not a ready-made latency percentile.

## Layout

```text
src/       Text, history, documents and project discovery
tests/    Correctness and deterministic randomized reference tests
benches/  Dependency-free benchmark harness
docs/     Contracts, limitations and measurement methodology
results/  Raw measurements, provenance and reconciled summaries
```

## Next gates

1. Correctness on Linux, Windows and macOS CI.
2. Repeated benchmark baselines and identified bottlenecks.
3. Balanced piece/line indexes and stable API contracts.
4. Only then reconsider any user interface.

## License

MIT. The library can run without GitHub; repository hosting and CI are replaceable.
