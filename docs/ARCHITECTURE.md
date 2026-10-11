# Architecture and contracts

## Text engine

An immutable original string plus append-only added text and a vector of pieces.
Every piece identifies a UTF-8-aligned range in one backing string. Adjacent
contiguous pieces are coalesced. Edits rewrite the piece vector, not the entire
text. Undo stores replacement deltas, not complete document snapshots.

- Public offsets are UTF-8 **byte** offsets, not UTF-16 units or grapheme indexes.
- Validation errors do not mutate text, revision or history.
- Batches use coordinates from the original pre-batch document. Overlapping
  ranges and edits with the same start are rejected. A batch is one undo step.
- Cursor columns count Unicode scalar values; combining characters and emoji
  clusters may occupy multiple columns. Lines split at LF; CRLF bytes are preserved.
- Revisions increase after real changes, undo and redo; no-op edits do not increment.
- History has an 8 MiB default payload budget, excluding collection overhead.
  A change larger than the budget clears prior history and is not undoable.
- Deleted added-text storage is reclaimed only by explicit `compact()`. Compaction
  leaves delta history valid and does not change revision or logical content.

## Costs and known limits

The piece vector is **not balanced**: locating/splitting pieces is O(piece count).
Sustained scattered inserts fragment it. Added-text storage can grow even when
history is bounded. `text_capacity_bytes` excludes piece/history/allocator costs
and is not process RSS. Cursor mapping is O(document bytes); cached line count
is O(1). Search materializes the document and returns non-overlapping literal
matches. Multi-edit batches materialize their affected span. These are explicit
optimization targets, not claims of a production-ready engine.

## Documents

Reject invalid UTF-8 and files larger than 16 MiB, with a bounded read rather
than trusting metadata. Preserve BOM and line endings. Dirty state compares
streamed text to the last saved contents. Save writes chunks to an exclusively
created file beside the destination, copies permissions, syncs the staged file,
rechecks external contents and renames it into place.

The checks are not a filesystem lock: a concurrent writer can race the final
check. Atomic replacement depends on filesystem/platform semantics; directory
sync and power-loss durability are not promised. Hard links, ACLs, extended
attributes and ownership may not survive replacement. Symlink targets are resolved
at open. Missing files and external changes are not silently recreated/overwritten.

## Project discovery

Iterative bounded directory traversal; skip symbolic links and common generated
directories. Return sorted file paths, explicit truncation and skipped-entry
counts. A truncated subset may differ with filesystem enumeration order. This is
synchronous discovery, not a background watcher or semantic index.

## Independence

Standard library only. No existing IDE code or proprietary assets. No language
server, executable project scripts or network service is invoked by this crate.
