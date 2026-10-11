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
  byte ranges and lines. For delayed consumers, enable the bounded change journal;
  a gap requires resynchronization from a fresh snapshot. Own scheduling/coalescing.
- Use chunks/find_iter to avoid unnecessary contiguous copies. Materialize only
  where a dependency requires an owned contiguous string.
- Keep document saving and project traversal outside latency-sensitive UI loops;
  this crate does not provide an async runtime or automatic background scheduler.
- Release old snapshots promptly; configure logical-byte and step limits, and account
  for structurally shared history roots. memory_stats
  is diagnostics, not a constant-time getter or total process memory limit.

## Stability

Version 0.3 is an early API, not a permanent compatibility promise. The existing
byte-range editing, search, save and cursor methods remain. EditBatch, Snapshot,
Version, ChangeSummary and MemoryStats make future integrations explicit.
Coordinates, ownership, errors, revision checks and memory semantics are documented;
"supports every future tool" is an extensibility goal, not implemented language tooling.

## Explicit position encoding

```rust
use kedr_core::{PositionEncoding, TextBuffer};
let buffer = TextBuffer::new("a🌲b");
assert_eq!(buffer.byte_to_position(5).unwrap(), (0, 2));
assert_eq!(buffer.byte_to_position_in(5, PositionEncoding::Utf16).unwrap(), (0, 3));
assert!(buffer.position_to_byte_in(0, 2, PositionEncoding::Utf16).is_err());
```

All edit ranges still use UTF-8 bytes. CRLF/BOM bytes remain unchanged. A CR counts
as a scalar/code unit before LF; protocol-specific line-ending rules belong in the
adapter. This API is not an LSP server or grapheme cursor implementation.

## Bounded invalidation journal

```rust
use kedr_core::TextBuffer;
let mut buffer = TextBuffer::new("abc");
buffer.set_change_log_capacity(256);
let observed = buffer.version();
buffer.replace(1..2, "🌲").unwrap();
let changes: Vec<_> = buffer.changes_since(observed).unwrap().collect();
assert_eq!(changes.len(), 1);
```

The journal is disabled by default. Capacity bounds retained entries, not allocator
bytes; zero frees its storage. Wrong-document, future-revision and evicted-event
requests return explicit errors. Each old_range is in its own old_revision's byte
coordinates. These are invalidations, not replayable edit payloads. take_change
only consumes the separate latest-change slot. No-op edits and compaction add no event.

## Prepared compaction

```rust
use kedr_core::TextBuffer;
let mut buffer = TextBuffer::new("abc");
buffer.replace(1..1, "🌲").unwrap();
let snapshot = buffer.snapshot();
let prepared = std::thread::spawn(move || snapshot.compacted()).join().unwrap();
buffer.install_compacted(prepared).unwrap();
```

If editing advances before installation, discard the stale result or recompute; do
not retarget it. No thread pool or automatic compaction scheduler is provided.
History and other snapshots can retain old storage. Dropping the replaced tree
can require reclamation work; installation is not a hard real-time guarantee.

## Revision exhaustion

Real edits and populated try_undo/try_redo return RevisionExhausted at u64::MAX
without mutation. Legacy bool undo/redo return false on that error; use the typed
methods to distinguish an empty stack from exhaustion. No-op edits remain valid.
