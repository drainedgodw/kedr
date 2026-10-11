# Validation

Tests cover UTF-8 edit boundaries, cross-piece replacements, delta history,
redo invalidation, budget eviction, no-op revisions, compaction, cursor round trips,
search across pieces and all-or-nothing edit batches. Eight deterministic random
seeds each perform 1,000 edits against a standard String reference, check cached
line counts and undo every real edit, with interleaved compaction.

Filesystem tests cover BOM/CRLF/Unicode round trips, dirty state, external changes,
binary/oversized inputs, missing files, concurrent fixture uniqueness, generated
folder exclusions, traversal limits and Unix symlink cycles.

CI is configured for Linux, Windows and macOS. Configuration alone is not proof
of a successful run. Local results and actual remote job results must be recorded
separately. The Unix-specific symlink test does not run on Windows.

No unsafe Rust is used. Miri, long-running fuzzing, crash-injection saves, race
stress with independent writers, ACL preservation and network-filesystem behavior
have not been validated. Performance tests do not substitute for correctness tests.

## Initial local evidence

Linux x86_64, Rust 1.99.0: `cargo fmt --check`, all 22 tests, and
`cargo clippy --all-targets -- -D warnings` passed. Release benchmark build passed.
Three process runs completed; 24 cases x 40 measured batches x 3 runs = 2,880 rows.
All 24 case medians were independently reconciled. This records local evidence,
not yet GitHub Actions success or Windows/macOS execution.

## Version 0.2 local checks

35 tests pass in debug and optimized release on Linux x86_64 / Rust 1.99.0.
All previous reference and filesystem tests remain. New tests cover immutable
snapshot lifetime/cross-thread reads, stale/wrong-document batch rejection,
no-op transactions, change envelopes, long Unicode checkpoint round trips,
streaming search, history step/redo pruning and deduplicated allocation accounting
including inline chunk headers. An internal stress test
checks every AVL node's balance/height/aggregate metrics after each of 4,096
fragmented Unicode inserts and all 4,096 undos. This is not long-running fuzzing.

Formatting, Clippy with warnings denied, Rustdoc with warnings denied and the
background-worker example pass locally. Remote platform/MSRV jobs are separate
and must be checked for the new commit, not inferred from old successful jobs.
