# Independent behavior reimplementation

## Scope and evidence policy

The goal is an independently authored Rust implementation of useful workflows
found in JetBrains products, with measured improvements where evidence supports
them. This repository does not contain a completed reverse engineering of the
JetBrains suite. Public product behavior/docs are reference specifications, not
permission to copy proprietary implementation, branding or bundled assets.
No decompilation of a JetBrains binary or proprietary-source analysis was
performed for the navigation/design revision.

Work is accepted per user scenario: reference behavior, actual implementation,
positive/negative tests, native UI acceptance and performance evidence. A menu
entry, catalogue product or screenshot does not establish functional parity.

## Current reference observations

The public IntelliJ IDEA documentation describes a Ctrl+E recent-files popup
with dynamic filtering, recent locations and local history:
[Recent files and changes](https://www.jetbrains.com/help/idea/recent-files-and-changes.html).
CodeCedar now implements **session-local recent files and bounded back/forward
locations only**. Edited-only toggling, local-history snapshots, recent-location
previews and reverting previous text are not implemented. macOS Cmd+Alt+arrows
are Kedr assignments; OS-reserved shortcuts may need remapping.

[Source navigation](https://www.jetbrains.com/help/idea/navigating-through-the-source-code.html)
separates file navigation from structural/semantic navigation. CodeCedar has file,
line and saved-source Cargo diagnostic navigation; declaration/usage/super-type
resolution is not delivered by that work.

[Search everywhere](https://www.jetbrains.com/help/idea/searching-everywhere.html)
covers files, actions, classes, symbols, settings and Git. CodeCedar's file and
action palettes must not be called complete Search Everywhere: symbol/index/Git
providers and double-Shift activation are not implemented.

## Next implementation gates (not shipped features)

1. Language service: versioned document synchronization, cancellation, capability
   negotiation and UTF-16 conversion; real Rust analysis via an explicit external
   service, then diagnostics, definition, hover and completion. Test stale/out-of-
   order responses, server exit and Unicode. A server that can run build scripts
   needs a visible trust/lifecycle policy. Do not substitute lexical matches for
   semantic definitions or rename.
2. Refactorings: workspace edits, preview, per-document version checks, atomic
   save/rollback and conflicts. Start with semantic rename, not text replace.
3. Build/test/debug: configurations, streaming test trees, durable stop/restart,
   process-tree lifecycle and a tested debug-adapter boundary.
4. Git/terminal: read-only status/diff first; explicit guarded mutations later.
   A PTY terminal is a separate component, not the Cargo Output text area.
5. Language/product adapters: prove Java/Kotlin, Python, JS/TS, Go, C/C++ and .NET
   workflows independently before declaring any corresponding catalogue product
   runnable. Sharing a shell does not implement those languages.
6. Specialized hosts: database transactions/secrets, profiling/symbolization,
   server persistence/auth/audit, remote permissions/sync and extension isolation
   require independent backend and interface acceptance. Existing catalogue
   entries remain planned until those gates pass.
7. Performance: compare identical projects, hardware, warm/cold caches and
   workloads. Report startup, typing/navigation p50/p95/p99, memory and long-session
   stability, with raw evidence. Rust alone is not proof of being faster.

See [parity gates](PARITY.md), [design contract](../studio/DESIGN.md) and
[validation](../studio/VALIDATION.md) for the current implemented boundary.
