# Product direction

## The product

Kedr Studio is the first flagship: a calm, native development workspace that grows
from a trustworthy editor into a complete project environment. Rust/Cargo is the
first integrated toolchain because Kedr can develop and test its own foundation.
Other text files can be edited now; that is not language-level IDE support.

The promise is local ownership, no paid activation and inspectable source. The
ambition is serious language/project tooling, not a collection of visually similar
editors advertised as complete IDEs. A large product needs many reviewed releases;
this preview is one vertical slice, not years of completed development.

## One repository, one interface foundation

- Root package: `kedr-core`, GUI-independent text/storage/document services.
- `crates/kedr-workbench`: editor actions, workspace model, search and task services.
- `crates/kedr-ui`: shared tokens, widgets, viewport editor, chrome, panels and dialogs.
- `apps/studio`: native eframe host, folder picker, bounded background queue and preferences.
- `products/studio.md`: installation/download guide for the real product.

The core stays standard-library-only. Desktop UI deliberately uses open-source
third-party egui/eframe/rfd libraries; the window/GPU stack is not claimed to be
written from scratch. There is no Electron/webview layer. No new language-product
crate should duplicate the shell or exist merely to change a title.

## Release gates, not invented delivery dates

1. **Native workbench (this preview).** Open/edit/find/save/close safely; discover
   commands; show real task output; validate visible states and bounded rendering.
2. **Language services.** Versioned/debounced diagnostics, completion, definitions,
   references and rename; transport caps/cancellation; stale results rejected;
   integration tests with a real language server and useful failure states.
3. **Project intelligence.** Incremental file watching/indexes, symbols, accurate
   persisted sessions/recovery, dirty-buffer project search and robust async saves.
4. **Execution and debugging.** Process-tree cancellation, terminal/PTY lifecycle,
   debugger adapter protocol, breakpoints, variables and execution configurations.
5. **Multi-language capability providers.** Real semantic/toolchain integrations;
   capability-driven commands and panels; common UI without unsupported empty tools.
6. **Extensibility and distribution.** Permission-scoped extensions, crash reports
   only with consent, packaging/signing/update verification and upgrade migrations.
7. **Production quality.** Real hardware latency/memory profiling, long interactive
   sessions, accessibility and IME coverage, crash/save/recovery fault injection.

Each gate needs evidence before being called done. No fabricated diagnostics,
completion suggestions, branch status, terminal sessions or benchmark superiority.
