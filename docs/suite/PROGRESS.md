# Delivered work

## Implemented

- Independent standard-library-only Rust text core: persistent AVL storage,
  Unicode coordinate conversion, guarded edits, snapshots, undo/redo and search.
- Workbench model: projects, tabs, UTF-8 editing, find/replace, bounded saved-file
  search, guarded saves and explicit Cargo task trust.
- Shared native UI: explorer, tabs, palettes, two themes, settings and viewport
  rendering. Native host uses open-source egui/eframe/rfd, not a custom OS stack.
- Original Studio release: 47 core + 14 workbench + 8 UI tests, 69 total; eleven
  native UI states were locally inspected. Additional changes need new evidence. The current keyboard revision passes 80
  local tests: 47 core + 18 workbench + 3 registry + 12 UI.
- Windows/macOS tests and release artifacts succeeded in first Studio CI run.
  Linux compiled/tested successfully but capture lacked libxkbcommon-x11; the
  dependency correction was published separately. See validation for actual runs.

## This catalog revision

Two persisted platform-aware keyboard profiles, 26 stable implemented action IDs,
searchable keyboard settings, correct focus dispatch and real line/navigation
commands were added. Unsupported modified edit keys no longer silently edit text.

The existing workbench is named CodeCedar; `kedr-studio` remains its package/binary.
Product names, original SVG marks, individual platform guides and acceptance plans
are established for the broader ecosystem. Planned entries are not working apps.

## Not delivered

Full semantic IDE tooling, debugger, terminal, Git UI, product-specific backends,
server services, extension sandbox, production recovery and JetBrains parity.
Logos/catalog entries/shortcut mappings do not implement those capabilities.

[Desktop evidence](../studio/VALIDATION.md) · [Parity gates](PARITY.md)
