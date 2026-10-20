# Workbench gap audit

**Current comprehensive review:** [90 findings across all 33 catalog products](PRODUCT_AUDIT.md),
with a [machine-readable backlog](product-audit.json). Only one runtime exists;
32 planned entries are explicitly NOT_IMPLEMENTED, not runtime-test passes.

The preceding recovery run [39305573342](https://github.com/drainedgodw/kedr/actions/runs/39305573342)
has now been observed successful across Linux, Windows (including installer), macOS
and Arch. The failure discussed below belongs to the older reload revision.
New audit fixes require their own hosted acceptance.

Audit basis: the actual CodeCedar source and acceptance tests, not the 33 catalog
names. Only CodeCedar is runnable. The rest are product plans; a new logo, command
binding or screenshot does not constitute an IDE implementation.
The preceding `reload` revision passed Linux, macOS and Arch jobs, but its
[Windows installer job failed](https://github.com/drainedgodw/kedr/actions/runs/39282097026/job/118158262423)
when the Chocolatey feed could not supply NSIS. Windows tests, live LSP, Clippy
and release compilation passed; installer acceptance did not. This revision adds
bounded install retries and explicit dependency/payload failure checks. Its new
hosted result must be checked separately.

## Findings and order of work

| Priority / journey | What works | Remaining risk / acceptance gate |
| --- | --- | --- |
| P0 — do not lose edits | Guarded staged saves, explicit dirty close/switch/quit, immutable saved baselines, active-file polling/reload and a [bounded plaintext draft journal with explicit restore/delete](../studio/RECOVERY.md) | Per-keystroke/power-loss durability, encrypted retention and full-session recovery are absent. Saves still block the UI. Save-all is not a cross-file transaction. No writer lock or external-write atomicity guarantee. Add fault injection for interruption, permissions, full disk and recovery |
| P0 — reconcile outside changes | Content-based active-file checks, changed/missing/unreadable notice, explicit Reload from disk; failed or stale reload keeps edits | Not a recursive file watcher: inactive files/new/deleted project entries are not refreshed. No merge/diff/Save As for conflicting edits; reload explicitly discards that file's edits and undo. Slow filesystem reads have no hard timeout |
| P1 — usable language intelligence | Real on-demand Rust definition and references requests, unsaved overlays, separate execution trust, UTF-16 and stale-result guards | A fresh analyzer for every action causes startup/indexing latency. Build a persistent versioned session, didChange/didClose, restart/cancel and workspace lifecycle tests before completion, hover and live diagnostics |
| P1 — safe semantic editing | Core supports guarded edit batches; no semantic refactoring is advertised | Rename/workspace edits need preview, version checks for every file, atomic conflict decisions and rollback/recovery. Do not wire F2 to textual replacement |
| P1 — ordinary project work | Existing scanned files open/edit; bounded saved-file search; Cargo check/test and diagnostics | New file/folder/rename/delete, rescan/index updates, unsaved project search, run configurations and test tree still need implementation. Cargo task output is not a PTY |
| P1 — process lifecycle | Explicit Cargo/analyzer permission; direct-child cancellation and bounded language transport | Descendant process-tree cancellation, output exit-drain ordering, isolation and denied executable/error paths require more target-platform tests |
| P2 — Git/debug/terminal | Not implemented | Start read-only Git status/diff; implement actual DAP/PTY adapters and lifecycle tests rather than dead menus |
| P2 — interaction quality | Dense shared dark/light shell, 37 stable actions, history/recent files, single-stroke remapping and keyboard result lists | Multi-stroke/double-modifier handling, broader focus contexts, persistent tool windows, localization and more complete source editing remain open |
| P2 — native accessibility | Partial labels and synthetic keyboard/IME tests | Custom editor lacks full assistive text/selection semantics; actual IME/keyboards, screen readers, font coverage, high DPI, Wayland and physical Windows/macOS acceptance remain open |
| P2 — release confidence | Multi-OS CI, native Linux capture fixtures and installer tests | CI is not physical-machine acceptance, code signing, reproducible updates or production stability. Run upgrade/recovery and long-session testing on real target systems |
| P3 — whole suite | Original names, marks and explicit planned guides | Every non-CodeCedar language/product/service needs actual providers/backends, security model and acceptance. Not a completed JetBrains replacement |

## Existing external-change reconciliation subset

Implementation: [file query/reader](../../crates/kedr-workbench/src/disk.rs),
[shared reload UI](../../crates/kedr-ui/src/shell/disk.rs),
[desktop background queue](../../apps/studio/src/jobs.rs).
The exact [file reconciliation contract](../studio/FILES.md) distinguishes active
polling, explicit reload, capture fixtures and the still-missing watcher/recovery.

Acceptance requires content changes (including equal-length rewrites), deletion,
invalid UTF-8, oversized/non-regular targets, escaped symlinks, unsaved text,
edit/undo/save/reopen/workspace invalidation, successful reload with Unicode/CRLF,
unchanged other dirty tabs, failed-read preservation and keyboard cancellation.
A real filesystem/worker test complements model tests; synthetic native captures
establish layout only. See [validation](../studio/VALIDATION.md) for actual counts.

## Next implementation sequence

1. Extend the bounded recovery subset with full-disk/permission/interrupted-commit
   fault injection, retention/privacy controls and physical-platform crash testing;
   move save I/O and large snapshot materialization off the render thread without
   accepting stale completions.
2. Persistent cancellable Rust LSP with versioned synchronization and crash/restart
   tests; then completion, hover, live diagnostics and a definition chooser.
3. Conflict diff/merge/Save As and project rescan/file operations; semantic rename
   only after safe multi-file edit/recovery contracts exist.
4. Real Git/PTY/DAP adapters and target-platform/accessibility/performance QA.

No percentage of overall parity or performance advantage is inferred from test
counts, a visually similar shell, or historical text-core microbenchmarks.

## This revision: recovery and distribution reliability

The [recovery contract](../studio/RECOVERY.md) specifies bounded independent session
journals, background I/O, explicit buffer-only restoration, stale/dirty-tab guards,
separate snapshot deletion and approved-quit retirement. Source snapshots remain
until explicitly deleted. Child-process termination and worker/UI model tests
complement labelled layout captures; this is not complete crash/power-loss
acceptance. Only CodeCedar is runnable. No new Git/PTY/DAP/LSP parity is inferred.

## Adversarial audit corrections

Four before/after regressions exposed real defects: Unix save staging permissions,
saving beyond the reader's 16 MiB bound, observed symlink/parent-path rebinding,
and deleted recovery snapshots resurfacing from retired revisions left after
failed cleanup. Fixes add private staging, bounded/identity-checked saves, and
ordered recovery deletion that leaves newer/temporary/unrelated entries alone.
These remain best-effort filesystem checks, not hostile-TOCTOU protection or
secure erasure. Added product inventory rejects missing runtime evidence and
multiple product names for the same host. The MSRV workspace check is a separate
CI gate, not an assertion of universal platform compatibility.
