# Integrating the core

Start with `examples/background.rs` and the generated Rustdoc:

```bash
cargo run --example background
cargo doc --no-deps --open
```

## Suggested boundaries

- Editor/controller owns one mutable TextBuffer per document.
- Parsers, search workers and indexers receive a Snapshot and its Version.
- A worker proposing a refactoring returns EditBatch::new(&snapshot, edits).
- The controller applies it only against that exact document/version. A rejection
  means re-read/recompute; never silently retarget offsets to newer text.
- After a successful change, inspect/take ChangeSummary to invalidate affected
  byte ranges and lines. If several edits occurred, own an event queue/coalescer.
- Use chunks/find_iter to avoid unnecessary contiguous copies. Materialize only
  where a dependency requires an owned contiguous string.
- Keep document saving and project traversal outside latency-sensitive UI loops;
  this crate does not provide an async runtime or automatic background scheduler.
- Release old snapshots promptly; configure logical-byte and step limits, and account
  for structurally shared history roots. memory_stats
  is diagnostics, not a constant-time getter or total process memory limit.

## Stability

Version 0.2 is an early API, not a permanent compatibility promise. The existing
byte-range editing, search, save and cursor methods remain. EditBatch, Snapshot,
Version, ChangeSummary and MemoryStats make future integrations explicit.
Coordinates, ownership, errors, revision checks and memory semantics are documented;
"supports every future tool" is an extensibility goal, not implemented language tooling.
