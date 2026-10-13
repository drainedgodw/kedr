# Workbench gap audit

This audit is against the actual source, not the number of catalog entries.
All other product backends remain planned. The immediate goal is a trustworthy
shared editor, not a large collection of empty controls.

| User journey | Gap / risk | Decision |
| --- | --- | --- |
| Learn and customize commands | Presets were discoverable but read-only; conflicts could make destructive actions ambiguous | Implemented single-stroke remapping, recording, atomic conflicts, unbind/reset and persistence |
| Keep interaction responsive | Re-resolving all overrides for every label would repeat work each frame | Cache resolved dispatch/hints until the profile or override map changes |
| Keep changes safe | Save guards exist, but crash recovery, file watching and async save lifecycle are incomplete | Next correctness gate: recovery and external-change reconciliation, with fault-injection tests |
| Write real code | Basic editing/colors exist; diagnostics, completion and semantic navigation do not | Separate language service milestone, versioned results and trust policy before auto-running tools |
| Run/debug work | Cargo check/test exists; run configs, debugger and terminal do not | Real adapters and lifecycle tests, not dead menu entries |
| Work with Git | No status/diff/commit interface | Start with read-only status/diff, then explicit safe mutations |
| Trust a release | Linux X11 captures and multi-OS CI are not physical platform acceptance | Arch/Wayland and Windows/macOS manual checks, accessibility and IME acceptance remain open |

## Acceptance for this change

- Old preferences remain readable; overrides round-trip through serialization.
- Conflicts never mutate config; all action IDs/platform/profile namespaces validate.
- Remapped dispatch/hints agree; editor-only context stays guarded.
- Recording does not execute the action or save an assignment until Apply.
- Cancellation, unbind and reset are distinct operations.
- Dark/light and compact settings/editor-dialog states are visually inspected.

The remaining gates in [PARITY.md](PARITY.md) still apply. This is not a claim of
JetBrains feature parity, universal shortcut coverage or measured speed superiority.
