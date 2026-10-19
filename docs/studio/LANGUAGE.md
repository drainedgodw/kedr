# Rust semantic navigation — on-demand LSP adapter

These are two working semantic workflows, not a completed language-intelligence
platform. CodeCedar can request a Rust symbol definition from an explicitly
installed `rust-analyzer`, using the active unsaved Rust buffer and overlays for
all other open Rust files. It navigates to the first returned definition or
shows a selectable list of usages inside the scanned workspace. Selected jumps
are recorded in back/forward history.

## Use

Install a compatible rust-analyzer and make it available on the application's
PATH. For a rustup installation, `rustup component add rust-analyzer` is one
option. The desktop app does not download servers or install a toolchain.

Open a Rust file, place the caret on a symbol, then use Navigate → Go to
definition or F12. The binding is a Kedr default and is remappable; exact
JetBrains product/platform keymap parity is not claimed. The first request asks
for language-analysis permission. Choose Allow once or allow for this workspace
session. Cargo-task trust and language-analysis trust are separate; neither is
persisted across workspace switches or application restarts. Run → Cancel
language request cancels a pending operation. It can also be remapped.

## Find usages

Use Navigate → Find usages or Alt+F7 with the caret on a Rust symbol. The binding
is remappable and may be intercepted by the operating system/window manager.
The same separate language-analysis permission applies; Cargo permission alone
does not authorize the analyzer. This is `textDocument/references` with
`includeDeclaration: false`, not literal text search or a reference index built
by Kedr. Definition and usages requests cannot run concurrently in one shell.

Results appear in a flat modal list ordered by workspace-relative path, line
and UTF-16 column, with a source-line preview (at most 180 Unicode scalars).
Up/Down selects, Enter or clicking opens a result, Escape/Close dismisses it.
Coordinates in the list are 1-based; columns are UTF-16 code units. An empty
response has an explicit empty state. Opening a result preserves dirty buffers
and creates a back/forward checkpoint. Results remain bound to the original
source/caret/tab/document revisions; changing any of these discards the list.
No semantic data is cached across requests or workspace switches.

There is no silent truncation: a response exceeding 128 locations is rejected.
Closed target sources are loaded once per path off the UI thread, at most
32 files, 1 MiB each and 4 MiB total (in addition to bounded open overlays).
Invalid positions, an external target or a failed source read reject the whole
result rather than presenting a misleading partial list. External filesystem
changes during analysis or while the result modal is open are not watched: the
loaded source is a snapshot, not a guarantee of current disk contents. Reconcile
external changes and repeat the request. The adapter does not find unscanned
standard-library/dependency usages, provide grouping/filtering/pinning, or show
multi-project results. Modal snapshots are not a persistent usages tool window.

## Execution and consistency contract

- Nothing starts automatically on project open or capture mode. The server is
  resolved through normal executable/PATH rules and runs with the workspace as
  its current directory. This is explicit code execution, **not a sandbox**.
- Initialize options disable rust-analyzer build scripts, procedural macros and
  check-on-save. A trusted server may still run toolchain/metadata processes and
  read files; these options are not a security boundary. Server-initiated edits
  and commands are rejected. Configuration/progress requests receive limited
  non-mutating responses.
- Each definition or usages request currently starts an isolated short-lived server. The adapter
  waits for rust-analyzer's quiescent workspace status before semantic lookup.
  This deliberately avoids claiming a low-latency persistent LSP session.
- Open Rust files are sent through `didOpen` with current unsaved contents. Wire
  version 1 is correct for each fresh session. Process-local document identity
  and monotonic revision stamps are retained separately to validate the result.
  This is **not** a continuous `didChange` stream.
- Any observed edit (including undo), reopened/closed/new tab, source caret/tab
  switch or workspace switch invalidates the pending request. Results are not
  rebased or applied to newer text. Non-Rust dirty files must be saved first.
- UTF-16 is explicitly negotiated. Core UTF-16 conversion rejects invalid
  positions/surrogate splits rather than moving the caret to a guessed offset.
- Targets must resolve to canonical local files in the scanned workspace.
  Remote URIs, out-of-workspace paths and escaping symlinks are rejected. Standard
  library/dependency navigation outside that scan is not implemented.
- Existing dirty targets are never replaced by disk contents. Unopened targets
  are read off the UI thread. The active-file disk notice is polling, not a file watcher or semantic workspace guard: external changes on disk
  during analysis can still make locations stale. Retry after reconciling files.
- Full overlays are limited to 1 MiB per Rust file, 4 MiB total and 32 files.
  Framed JSON messages have a 4 MiB bound, headers 8 KiB, incoming/outgoing queues 8/64 messages and
  each response wait a 4096-message budget. Excess input is rejected. Each protocol query
  has a 30-second deadline, including initial indexing; bounded preview loading
  afterwards remains cancellable.
- Cancellation sends a best-effort `$/cancelRequest`, kills and reaps the direct
  server child, with a short graceful shutdown on success. Descendant process
  tree cancellation/isolation is not provided. Stderr is discarded, not exposed
  as an unbounded UI log.
- Multiple definition results currently select the first and report the number
  of alternatives. A definition chooser is not implemented. Usages instead show
  all accepted results for explicit selection.

## Evidence and next gates

Protocol fixtures cover framing, malformed input, null/error responses, encoding
rejection, unrelated IDs, unsaved overlays, denial of server edits, timeout and
cancellation. Model/UI tests cover trust separation, history, dirty target
preservation and invalid UTF-16. A separate live rust-analyzer test resolves the
real two-file Cargo fixture with unsaved source **and** target contents and
checks the shifted declaration coordinates. It is explicitly run in CI after
installing rust-analyzer, not silently counted as part of default unit tests.
A second explicitly run live test finds multiple real Garden constructor usages
at positions shifted by an unsaved source overlay and excludes the declaration.
Usages regressions cover capability negotiation, array/null responses, cancelled
work, deduplication/order, preview bounds, UTF-16 validation, separate permission,
stale selection and preservation of dirty buffers. Static native capture scenes
are visibly labeled fixtures, never presented as live-analysis evidence.

Next: a persistent versioned/cancellable session with `didChange`/`didClose`,
hover, completion, diagnostics, safe workspace-edit previews and a
multiple-target chooser. No rename/refactoring, debugger, Git or full suite
parity is delivered by this first adapter.
