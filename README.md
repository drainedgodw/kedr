# Kedr

Open-source, local-first development tools. **Kedr Studio is the first native desktop
preview**, built on the independently developed Rust text core. MIT licensed; no
Kedr account, telemetry, paid activation or runtime cloud service required.

## Studio

A calm shared workbench: Midnight/Daylight appearance, file explorer, tabs, quick
navigation, command palette, viewport-rendered editor, find/replace, saved-project
search, safe-save guards and explicitly trusted Cargo check/test tasks.

This is an early editor/workbench, **not a complete JetBrains replacement**. Language
server completion, semantic refactoring, debugger and terminal are still ahead.

[Download/build guide](products/studio.md) · [Product direction](docs/studio/PRODUCT.md) ·
[Shared design contract](docs/studio/DESIGN.md) · [Validation](docs/studio/VALIDATION.md)

```bash
cargo run --locked --release -p kedr-studio -- /path/to/project
```

[Platform preview artifacts](https://github.com/drainedgodw/kedr/actions/workflows/studio.yml)
are available only when the corresponding job actually succeeds. They are unsigned
previews, not stable installers. Linux X11 was inspected locally; physical Windows,
macOS and Wayland interaction testing remains separate.

## One repository, one shared interface

```text
src/                     GUI-independent Rust core (standard library only)
crates/kedr-workbench/    Editing workflows, workspace/search/task services
crates/kedr-ui/           Shared theme, chrome, panels, dialogs and viewport editor
apps/studio/             Native eframe host, folder picker and background queue
products/                Download/build guides for actual products
```

Future products reuse the whole `kedr-ui` shell rather than copying or merely
recoloring separate interfaces. Only Studio exists today. Desktop presentation
uses open-source egui/eframe/rfd; no Electron/webview. The GUI toolkit/OS stack is
not claimed to have been written from scratch.

## Core

Persistent AVL text storage, byte/scalar/UTF-16 rank/select, root-sharing undo/redo,
immutable snapshots, version-guarded edit batches/prepared compaction, bounded
invalidation journal, streaming search and reachable-memory diagnostics.

```bash
cargo test --locked -p kedr-core
cargo test --locked --workspace
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo bench --locked --bench core
```

[API](docs/API.md) · [Architecture](docs/ARCHITECTURE.md) ·
[Core 0.3 endurance](docs/HARDENING.md) · [Historical core comparison](PERFORMANCE.md)

Core benchmarks are historical source-fingerprinted evidence, not Studio/IDE
benchmarks. Gains and regressions are both reported. Default workspace commands
remain scoped to the core; use `--workspace` for all product crates.
