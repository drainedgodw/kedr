# Delivered work

## Implemented

- Independent standard-library-only Rust text core: persistent AVL storage,
  Unicode coordinate conversion, guarded edits, snapshots, undo/redo and search.
- Workbench model: projects, tabs, UTF-8 editing, find/replace, bounded saved-file
  search, guarded saves and explicit Cargo task trust.
- Shared native UI: explorer, tabs, palettes, two themes, settings and viewport
  rendering. Native host uses open-source egui/eframe/rfd, not a custom OS stack.
- Original Studio release: 47 core + 14 workbench + 8 UI tests, 69 total; eleven
  native UI states were locally inspected. Additional changes need new evidence. The preceding keyboard revision passed 93
  local tests. That earlier desktop revision passed 105 Rust tests plus 18 Python
  installer tests; subsequent acceptance entries record later counts and platform runs.
- Windows/macOS tests and release artifacts succeeded in first Studio CI run.
  Linux compiled/tested successfully but capture lacked libxkbcommon-x11; the
  dependency correction was published separately. See validation for actual runs.

## This catalog revision

Two persisted platform-aware keyboard profiles (originally 26 stable implemented action IDs),
searchable keyboard settings, correct focus dispatch and real line/navigation
commands were added. Unsupported modified edit keys no longer silently edit text.
Single-stroke custom bindings now support recording, atomic conflict checks,
unbinding/reset, separate platform/profile overrides and persistent preferences.
Resolved bindings are cached until configuration changes.

The existing workbench is named CodeCedar; `kedr-studio` remains its package/binary.
Product names, original SVG marks, individual platform guides and acceptance plans
are established for the broader ecosystem. Planned entries are not working apps.

## Not delivered

Full semantic IDE tooling, debugger, terminal, Git UI, product-specific backends,
server services, extension sandbox, production recovery and JetBrains parity.
Logos/catalog entries/shortcut mappings do not implement those capabilities.

[Desktop evidence](../studio/VALIDATION.md) · [Parity gates](PARITY.md)

## Installation and dialog revision

Original identities were redesigned with split-cedar geometry, larger outlined
monograms and per-product accents. CodeCedar now supplies Linux menu/hicolor
icons, native window identity and a Windows executable/resource/setup icon.
An offline terminal selector lists all products while allowing only CodeCedar.
The per-user XDG installer validates payload/ownership, upgrades, verifies and
uninstalls; projects/preferences and modified files are protected on Linux.
A local VCS PKGBUILD uses makepkg/pacman without AUR publication. A per-user NSIS
wizard registers Windows Start menu icons and an Apps-settings uninstaller.

A reported folder-picker freeze exposed a blocking native dialog call on the
render thread. Selection is now isolated on a dedicated worker with one pending
dialog, cancellation/failure handling and generation checks for stale results.
This fixes the architectural blockage; physical Wayland acceptance is still open.

## Dense desktop / editing revision

The header, explorer, controls and tab strip were compacted; File/Edit/Navigate/
Search/View/Run menus expose implemented commands instead of placeholders. Four
new actions add textual line comments, move-line/block up/down and join-next-line
operations. All are editor-focus-only, discoverable/remappable, Unicode-safe and
undoable. Comment syntax is explicit by extension, unsupported types fail visibly;
large edits are bounded rather than flattening the entire document.

The viewport layout cache now prunes distant rows while retaining nearby galleys
instead of clearing everything after crossing 256 cached rows. A 100,000-line test
exercises distant jumps and asserts limited painted/cached rows. No unmeasured
percentage speedup or optimization of nonexistent product backends is claimed.

## Current navigation and file-reconciliation subset

On-demand Rust definition and usages navigation now use an explicitly authorized
rust-analyzer, open unsaved overlays and version guards, with real integration
tests. They are not persistent LSP or completion/refactoring. The shared registry
now has 37 implemented actions, including confirmed Reload file from disk.

The native host queues bounded active-file content checks off the UI thread and
shows changed/missing/unreadable notices without replacing edits. Confirmed reload
checks document identity/revision/saved baseline before accepting its background
result; other dirty tabs remain intact. Crash recovery, merge/diff, async saves and
recursive watching remain open. See [current audit](AUDIT.md) and
[file contract](../studio/FILES.md), not historical test totals, for remaining work.
