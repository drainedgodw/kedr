# Shared Kedr interface contract

The interface is a reusable shell, not a screenshot copied into each application.
Every future product uses `kedr-ui::Shell` and the same theme tokens/components.

## Visual language

Midnight and Daylight: quiet neutral surfaces, one blue accent, restrained borders,
8-pixel controls, 12-pixel dialogs, 14-pixel interface text and 15-pixel code by
default. The spacing scale is 4/8/12/16/24/32/48/64. Icons are native vector strokes,
not missing-font pictograms. Success/warning colors retain textual labels.

The primary reading area is code. Avoid glowing gradients, decorative cards,
product logos in code panes or dashboards of unrelated statistics. Leave room
between actions. A selected file/tab must be clear without relying on color alone.

## Stable interaction vocabulary

- Top: brand, workspace, File/Edit menus, file/command search and real toolchain action.
- Left rail: Files, saved-project Search, Commands, Output and Settings.
- Explorer: left-aligned hierarchy; active file visible; resizable sidebar; bounded rows.
- Center: tabs, breadcrumb and viewport-rendered editor; no full-buffer TextEdit copy.
- Bottom: real task output and compact document position/encoding/save state.
- Modal decisions: palette, appearance, unsaved edits and explicit project trust.

Quick file navigation and commands are distinct modes of the same discoverable
palette. Destructive close/switch/quit paths preserve edits until a user decision.
No project command runs on open. Trust is session-scoped and not silently persisted.

## Native and responsive scope

Desktop window minimum: 760x520. QA includes 1440x900 and 900x640, both appearance
themes, palette/settings/search/trust/unsaved/error/welcome states. This is not a
mobile application or browser mock. Native folder-picking and actual OS-level IME/
screen-reader usage still require manual checks on target machines.

Ordinary controls have 44-pixel interaction targets and accessible names. The custom
editor is one large keyboard/pointer target with clipboard/composition events;
complete assistive text semantics are a later release gate, not an asserted feature.
