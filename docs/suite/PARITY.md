# Feature and shortcut parity gates

This is a development/acceptance plan, not a completed-feature claim. A category
reference to a JetBrains product never implies compatibility, affiliation or
feature parity. Work is tracked per actual capability, not per renamed binary.
The concrete [independent reimplementation contract](REIMPLEMENTATION.md) separates
reference observations, shipped subsets and next implementation gates.

| Capability family | Current evidence | Required next gate |
| --- | --- | --- |
| Editing / local workspace | Early editor, guarded saves, bounded search, active-file disk notices, confirmed background reload and bounded local draft recovery | Durable recovery/fault-injection expansion, async saves, recursive watching/rescan, conflict merge and fault injection |
| UX / keyboard | Shared desktop shell, bounded back/forward, recent files and conflict-safe single-stroke remapping | Multi-stroke/modifier gestures, broader context dispatch, target-machine QA |
| Language intelligence | Basic line colors, saved-source Cargo diagnostics, and [on-demand Rust LSP definition/usages navigation](../studio/LANGUAGE.md) | Persistent versioned LSP, completion, diagnostics, rename and parser tests |
| Refactoring / intentions | Not implemented | Semantic correctness, previews, cross-file transactions, rollback |
| Build / test / run | Cargo check/test with explicit trust | Toolchain adapters, run configs, output lifecycle, test trees and coverage |
| Debugging | Not implemented | DAP/native adapters, breakpoints, stepping, variables and integration tests |
| Terminal | Not implemented | PTY, shell lifecycle, process-tree cancellation and platform tests |
| Git / review | Not implemented | Status, diff, history, conflict resolution and safe mutation flows |
| Database workflows | Not implemented | Driver isolation, secrets, SQL analysis and transaction safety |
| Profiling / decompilation | Not implemented | Actual runtime collectors, symbol models and trace/heap analysis |
| CI / issue tracking / identity | Not implemented | Real service backends, persistence, auth, isolation and auditability |
| Remote / collaboration | Not implemented | Protocol, sync, permissions, recovery and threat-model tests |
| AI / plugins / distribution | Not implemented | Consent, capabilities, sandboxing, signed packages and revocation |
| Accessibility / IME / localization | Partial labels and synthetic IME | Native assistive text model, real keyboards/IMEs and localization tests |
| Performance | Historical core microbenchmarks only | Comparable hardware, latency distributions, memory and long-session profiling |

## Shortcut coverage policy

Every working command has one stable action identifier, one implementation and
platform/profile-aware discoverable bindings. Unsupported completion, intention,
refactoring, Git, terminal and debugger shortcuts stay in the parity backlog;
they are not mapped to unrelated commands. Double-Shift/Search Everywhere is not
claimed when only a file palette exists. Cargo check is not labelled Rebuild.

Multi-stroke chords, double-modifier handling and
complete context-specific keymaps are later gates. A compatible subset is not
"all JetBrains keybinds". Default maps differ by product, platform, version and
user configuration, so acceptance must specify a baseline.

## Shared interface scope

Use one theme/action/interaction contract. Desktop hosts reuse `kedr-ui`; service
web hosts need the same tokens and interaction conventions with appropriate
service-specific components. Do not clone unrelated desktop panels into a server
UI or create non-functional menu items to inflate apparent feature count.

## Reference baseline

- [JetBrains product catalog](https://www.jetbrains.com/products.html)
- [Windows/Linux shortcut reference](https://www.jetbrains.com/help/idea/reference-keymap-win-default.html)
- [macOS shortcut reference](https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html)
- [Keymap configuration](https://www.jetbrains.com/help/idea/settings-keymap.html)

Only individually tested capabilities graduate from planned to implemented.
