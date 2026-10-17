# Cargo diagnostics: implemented scope

CodeCedar now offers a real saved-source workflow:

1. Open a Cargo workspace and save source changes.
2. Choose Cargo check and explicitly allow the project to run when prompted.
3. Read the Problems tool window: error/warning, rustc code, source location and message.
4. Activate a result to open its scanned workspace file and go to its 1-based
   Unicode-scalar line/column. Existing tabs are reused. Unsaved workspace edits
   block this navigation; save and rerun Check. Source changes after the check,
   including externally modified or subsequently saved files, can make results stale.
5. Inspect Output for rendered compiler messages and ordinary process failures;
   Cargo Test automatically uses Output rather than pretending to provide structured
   test results. Check automatically selects Problems. The panel can be resized/hidden.

Only Cargo check stdout is interpreted as Cargo JSON. Stderr and Cargo Test stay
console text. Reads are framed before UTF-8/JSON decoding, including split Unicode
and final records without a newline. ANSI escape bytes are made visible rather
than interpreted. Cargo is launched directly, not through a shell.

Bounds: 1 MiB per framed record (oversize records discarded until next newline),
1,000 unique diagnostics, 4,096 Unicode scalars per message/path and 64 per code;
console tail at most 256 KiB/4,096 chunks. Diagnostic snapshots use Arc copy-on-write
rather than cloning every message each render. Problems renders only visible rows.
All canonical navigation paths must remain within the workspace and be in its
scanned file set; external files, missing paths and escaping symlinks do not open.
Primary spans only; diagnostics without source remain readable but not navigable.
No automatic dependency download, project execution or trust bypass is introduced.
An explicitly permitted Cargo invocation itself may download dependencies, as before.

This is not LSP, live diagnostics, a semantic project index, a test explorer,
refactoring support or JetBrains feature parity. Cancellation still targets the
Cargo child; complete cross-platform descendant-process containment is not claimed.

Screenshot states `problems` and `output` use a clearly labelled deterministic
capture fixture. They never run project code and must not be presented as evidence
of a real compiler invocation. A separate integration test compiles an authored,
broken Rust fixture and verifies an actual E0425 diagnostic and source location.
