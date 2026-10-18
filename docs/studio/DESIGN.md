# Shared desktop workbench design contract

This is an independently implemented Rust/egui shell, not JetBrains source code,
assets, an affiliation claim, or completed suite-wide feature parity. Only
CodeCedar currently hosts it as a runnable application.

## Surface and density

Use neutral graphite/white surfaces, a restrained blue selection/navigation
accent, textual error/warning states, 3px control corners and 6px modal corners.
Body/buttons are 13px; code defaults to 15px with user scaling. Ordinary controls
are 28px, icon targets 32px, header 44px, status 26px and explorer rows 28px.
This is a mouse/keyboard desktop density, not touch acceptance. Dialogs have 16px
body margins; palettes 12px. Do not use marketing cards, slogans, glows or a giant
brand header in the workspace.

## Hierarchy

- Flat File/Edit/Navigate/Search/View/Run menu bar; shortcut hints align on the
  right of menu items. Only implemented actions appear. Back/forward availability
  reflects the navigation history; Cargo actions require a scanned Cargo.toml.
- Tabs group file label, unsaved marker and close target. The active tab has a
  distinct canvas and underline. Middle-click closes through the same guarded
  close workflow. Tab names have full-path hover labels.
- The path strip is compact, with document language and relative path; its
  context menu copies either relative or absolute path. It is not a semantic
  symbol breadcrumb and is not presented as one.
- Explorer/search are resizable, clipped and bounded. Selected source stays
  distinguishable. The main reading area belongs to code.
- Bottom Problems/Output shows real Cargo diagnostics/output. It is a resizable
  tool window, not a terminal or a debugger.
- Settings, shortcuts, trust and unsaved-file decisions use the same typography,
  margins and neutral surfaces. Project execution still requires explicit trust.

## Lists and keyboard navigation

Go to file, Recent files and Find action use flat, left-aligned rows instead of
large card-shaped buttons. File rows show file name plus relative path. Action
rows reserve a separate right-aligned shortcut column, with clipped titles.
The selected row is singular; keyboard movement scrolls it into view without
continually fighting manual scrolling. Escape closes, Enter activates; disabled
actions cannot execute. Empty and filtered-empty states explain what happened.

Recent files are workspace/session-local, deduplicated, at most 50, most recent
first. Quick-open uses recent order for an empty query and fuzzy project-file
matching for a query. Navigation history is bounded to 128 points; ordinary
caret motion updates a point, while file switches/explicit location jumps make
checkpoints. Back/forward can reopen a cleanly closed file using background I/O.
It restores validated Unicode-scalar coordinates, not file snapshots or old text.
Unsaved buffers are retained. Invalid/missing locations report errors rather than
silently saving, discarding or guessing a location. A new branch drops forward
history. Switching workspaces clears this session-local navigation state.

## Start screen

Show product name, Open workspace, Find action, Settings and recent workspace
rows. No subscription slogan or oversized inspirational copy. A workspace with no
open file shows an actionable empty state with the actual configured shortcut.

## Validation and limits

Target minimum is 760x520; normal/compact QA is 1440x900 and 900x640 in both
appearance themes. Captures are native windows, not HTML mockups. Screenshots
establish layout observations only. Real OS-level IME, screen-reader text
semantics, visual acuity preferences, Windows/macOS native interaction and
Wayland behavior still need target-machine acceptance. This revision does not
complete the design of planned database, profiler, server or collaboration hosts.
Their real workflows need host-specific designs, not copies of IDE panels.
