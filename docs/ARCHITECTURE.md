# Architecture and contracts

## Indexed text storage

Persistent AVL piece tree. Immutable chunks hold text, adaptive 32/64-bit newline offsets and sparse
Unicode-scalar/UTF-16 checkpoints; ASCII chunks need no checkpoint array.
Edit payloads up to 24 UTF-8 bytes are stored inline; tiny chunks need no index allocation.
Branches cache byte/scalar/UTF-16/newline/piece counts and height. Split/join shares
unmodified subtrees and validates UTF-8 boundaries before mutation.

- Piece lookup/splice: O(log P) tree work, plus replacement indexing and reported history.
- Cursor mapping: tree rank/select plus backing-index binary searches and at most
  256 Unicode scalars scanned per checkpoint query. No whole-document cursor scan.
- New buffers still need O(text bytes) index construction. Metadata consumes memory.
- Snapshots are O(1) immutable root handles, Send + Sync, with no writer lock.
- A mutable TextBuffer clone shares text but clones history entries and receives a
  new process-local document identity. Use Snapshot for background readers.
- Literal search is lazy: native str search on one chunk, KMP across chunks.
  It does not flatten the document; fragmented-search constants can still differ.
- Slice iteration skips unrelated subtrees. to_text explicitly allocates contiguous text.

## Public coordinates and versions

Offsets are UTF-8 byte offsets. Existing cursor methods count Unicode scalars;
encoding-aware methods explicitly accept UnicodeScalar or Utf16 code-unit columns.
Neither counts graphemes; offsets inside a surrogate pair are rejected. Preserve BOM and CRLF bytes. Revisions increase on real content
changes and undo/redo; compaction does not change content revision. Version pairs
process-local document identity with content revision, not a persistent/global UUID.

Build EditBatch from the exact Snapshot read by a background worker. Applying it
rejects another document or stale revision before mutation. Batches use original
coordinates, reject overlapping/same-start edits and form one undo transaction.
No-op batches emit no change. ChangeSummary is the latest invalidation envelope.
An optional bounded journal retains summaries, not replacement text; it is disabled
by default and reports gaps explicitly. Consumers still own scheduling/coalescing.
Revisions never wrap: real changes at u64::MAX return RevisionExhausted before mutation.
Process-local document ID exhaustion panics instead of reusing an ID.

## History and retention

Undo/redo switches between structurally shared version roots in O(1), not flattened text copies.
The default limits are 8 MiB of logical affected bytes and 4,096 steps.
set_history_step_limit adjusts the latter. The byte budget includes a
batch's affected span on both sides of the change. It is **not** a physical-memory cap. A small range can pin a
large immutable backing chunk. Oversized changes clear older history and are not
undoable. clear_history releases retained undo roots; compact consolidates only the
live root. Snapshot::compacted can prepare a replacement root on a worker;
install_compacted only accepts the exact current document/version. This avoids
rebuilding indexes on the controller, but does not promise constant-time destruction
of the displaced root. External snapshots/history can keep old storage alive after compaction.

memory_stats traverses and deduplicates nodes/chunks reachable from this buffer
and its history. It excludes externally held snapshot roots, allocator overhead,
thread stacks and unrelated process allocations. Shared bytes are counted once
per call but are not exclusively owned. It is O(reachable nodes), not a hot-path getter.
Linux VmHWM measurements in the performance report are whole-process peak RSS.

## Documents and projects

Document reading/saving and bounded project discovery remain separate from text.
Reject invalid UTF-8 and documents larger than 16 MiB. Staged save syncs a file,
rechecks external text and renames beside the destination. This is not a writer lock
or a power-loss/ACL/hard-link preservation guarantee. Project discovery is synchronous,
bounded, skips symlinks/generated folders and reports truncation/skipped entries.

## Boundaries

Standard library only; no GUI, network service or project script execution.
No parser, semantic index, terminal, debugger, plugin runtime, grapheme segmentation,
LSP server/transport or distributed collaboration protocol is implemented.
UTF-16 coordinate primitives are available, not a complete language-protocol adapter. These can consume
snapshots and validated batches without coupling language-specific logic to storage.
