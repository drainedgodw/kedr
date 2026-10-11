# Kedr

GUI-independent Rust editor core. **Core first; no IDE products or UI.**
Standard library only. No runtime service, paid activation, accounts or telemetry.

## Current foundation

- Persistent balanced AVL piece tree with indexed line/scalar/UTF-16 rank/select.
- UTF-8 byte edits, all-or-nothing batches and shared-subtree undo/redo.
- O(1) immutable snapshots for background readers.
- Document identity + revision checks for background edits and prepared compaction.
- Optional bounded invalidation journal with explicit resynchronization errors.
- Lazy cross-piece search and explicit text materialization.
- Document save protections and bounded project discovery.
- Reachable-memory diagnostics, correctness tests and before/after benchmarks.
- Million-operation endurance fixtures with individual latency and memory evidence.

There is no parser, semantic index, debugger, plugin runtime, UI or product catalog.
Improved microbenchmarks do not imply superiority over another IDE or all workloads.

## Develop

Rust 1.88+:

```bash
cargo fmt --check
cargo test --locked
cargo test --locked --release
cargo clippy --locked --all-targets -- -D warnings
cargo bench --locked --bench core > measurements.csv
cargo run --example background
cargo run --release --example endurance -- unicode 10000 results/local-endurance
cargo doc --no-deps --open
```

[API integration](docs/API.md) · [Architecture and limits](docs/ARCHITECTURE.md) ·
[Historical 0.1→0.2 comparison](PERFORMANCE.md) · [Benchmark methodology](docs/BENCHMARKS.md) ·
[Validation](docs/VALIDATION.md) · [0.3 hardening](docs/HARDENING.md)

The original baseline remains in `BENCHMARK_RESULTS.md`. New paired runs,
historical 0.1→0.2 source fingerprints and whole-process memory measurements are in `results/before/`,
`results/after/` and `results/comparison/`. See the report for **regressions as well as gains**. Version 0.3 evidence is in
`results/hardening/` and `docs/HARDENING.md`.

## Structure

```text
src/storage/   Immutable backing indexes and persistent AVL storage
src/           Text API, snapshots, versions, search, history, memory, documents/projects
tests/         Reference-model, filesystem and integration regressions
benches/       Dependency-free release workloads
examples/      Background-worker API and Linux peak-RSS fixture
results/       Raw measurements, environment/source fingerprints and reconciled evidence
```

## License

MIT. Hosting and CI can move; the library does not require GitHub to run.
