# Delivered work

## Implemented

- Independent standard-library-only Rust text core: persistent AVL storage,
  Unicode coordinate conversion, guarded edits, snapshots, undo/redo and search.
- Workbench model: projects, tabs, UTF-8 editing, find/replace, bounded saved-file
  search, guarded saves and explicit Cargo task trust.
- Shared native UI: explorer, tabs, palettes, two themes, settings and viewport
  rendering. Native host uses open-source egui/eframe/rfd, not a custom OS stack.
- Original Studio release: 47 core + 14 workbench + 8 UI tests, 69 total; eleven
  native UI states were locally inspected. Additional changes need new evidence. The current keyboard revision passes 93
  local tests: 47 core + 18 workbench + 10 registry + 2 dialog + 15 UI + 1 preferences.
- Windows/macOS tests and release artifacts succeeded in first Studio CI run.
  Linux compiled/tested successfully but capture lacked libxkbcommon-x11; the
  dependency correction was published separately. See validation for actual runs.

## This catalog revision

Two persisted platform-aware keyboard profiles, 26 stable implemented action IDs,
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
