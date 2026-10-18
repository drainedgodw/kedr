# Rust definition navigation — first LSP adapter

This is one working semantic workflow, not a completed language-intelligence
platform. CodeCedar can request a Rust symbol definition from an explicitly
installed `rust-analyzer`, using the active unsaved Rust buffer and overlays for
all other open Rust files. It navigates to the first returned definition inside
the scanned workspace and records the jump in back/forward history.

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

## Execution and consistency contract

- Nothing starts automatically on project open or capture mode. The server is
  resolved through normal executable/PATH rules and runs with the workspace as
  its current directory. This is explicit code execution, **not a sandbox**.
- Initialize options disable rust-analyzer build scripts, procedural macros and
  check-on-save. A trusted server may still run toolchain/metadata processes and
  read files; these options are not a security boundary. Server-initiated edits
  and commands are rejected. Configuration/progress requests receive limited
  non-mutating responses.
- Each F12 request currently starts an isolated short-lived server. The adapter
  waits for rust-analyzer's quiescent workspace status before definition lookup.
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
  are read off the UI thread. There is no file watcher: external changes on disk
  during analysis can still make locations stale. Retry after reconciling files.
- Full overlays are limited to 1 MiB per Rust file, 4 MiB total and 32 files.
  Framed JSON messages have a 4 MiB bound, headers 8 KiB, incoming/outgoing queues 8/64 messages and
  each response wait a 4096-message budget. Excess input is rejected. Each query
  has a 30-second deadline, including initial loading.
- Cancellation sends a best-effort `$/cancelRequest`, kills and reaps the direct
  server child, with a short graceful shutdown on success. Descendant process
  tree cancellation/isolation is not provided. Stderr is discarded, not exposed
  as an unbounded UI log.
- Multiple results currently select the first and report the number of
  alternatives. A definition chooser is not implemented.

## Evidence and next gates

Protocol fixtures cover framing, malformed input, null/error responses, encoding
rejection, unrelated IDs, unsaved overlays, denial of server edits, timeout and
cancellation. Model/UI tests cover trust separation, history, dirty target
preservation and invalid UTF-16. A separate live rust-analyzer test resolves the
real two-file Cargo fixture with unsaved source **and** target contents and
checks the shifted declaration coordinates. It is explicitly run in CI after
installing rust-analyzer, not silently counted as part of default unit tests.

Next: a persistent versioned/cancellable session with `didChange`/`didClose`,
references, hover, completion, diagnostics, safe workspace-edit previews and a
multiple-target chooser. No rename/refactoring, debugger, Git or full suite
parity is delivered by this first adapter.
