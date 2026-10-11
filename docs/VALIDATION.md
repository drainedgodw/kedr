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
