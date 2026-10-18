# Implemented keyboard profiles

Generated with `cargo run --locked -p kedr-ui --example keymap`. The same registry drives dispatch, menu/palette hints and the in-app Keyboard shortcuts screen.

The default profile is a **JetBrains-compatible subset**, not full keymap or feature parity. Kedr classic is selectable in Settings → Keyboard shortcuts; the choice is persisted locally.

| Working action / ID | Compatible Windows/Linux | Compatible macOS | Classic Windows/Linux | Classic macOS |
| --- | --- | --- | --- | --- |
| Navigate back / `navigation.back` | Ctrl Alt Left | Cmd Alt Left | Ctrl Alt Left | Cmd Alt Left |
| Navigate forward / `navigation.forward` | Ctrl Alt Right | Cmd Alt Right | Ctrl Alt Right | Cmd Alt Right |
| Recent files / `navigation.recent` | Ctrl E | Cmd E | Ctrl E | Cmd E |
| Go to definition / `navigation.definition` | F12 | F12 | F12 | F12 |
| Cancel language request / `language.cancel` | — | — | — | — |
| Show project files / `workspace.project` | Alt 1 | Cmd 1 | Ctrl 1 | Cmd 1 |
| Replace in file / `search.replace` | Ctrl R | Cmd R | Ctrl R | Cmd R |
| Find next occurrence / `search.next` | F3 | Cmd G | F3 | F3 |
| Find previous occurrence / `search.previous` | Shift F3 | Cmd Shift G | Shift F3 | Shift F3 |
| Next editor tab / `tab.next` | Ctrl Tab | Ctrl Tab | Ctrl Tab | Ctrl Tab |
| Previous editor tab / `tab.previous` | Ctrl Shift Tab | Ctrl Shift Tab | Ctrl Shift Tab | Ctrl Shift Tab |
| Select all / `edit.select_all` | Ctrl A | Cmd A | Ctrl A | Cmd A |
| Duplicate line or selection / `edit.duplicate` | Ctrl D | Cmd D | Ctrl Shift D | Cmd Shift D |
| Delete current line / `edit.delete_line` | Ctrl Y | Cmd Backspace | — | — |
| Toggle line comment / `edit.comment` | Ctrl / | Cmd / | Ctrl / | Cmd / |
| Move line or block up / `edit.move_up` | Alt Shift Up | Alt Shift Up | Alt Shift Up | Alt Shift Up |
| Move line or block down / `edit.move_down` | Alt Shift Down | Alt Shift Down | Alt Shift Down | Alt Shift Down |
| Join with next line / `edit.join_lines` | Ctrl Shift J | Cmd Shift J | Ctrl Shift J | Cmd Shift J |
| Go to line / column / `navigation.line` | Ctrl G | Cmd L | Ctrl G | Cmd G |
| Keyboard shortcuts / `settings.keymap` | — | — | — | — |
| Open workspace / `workspace.open` | Ctrl O | Cmd O | Ctrl O | Cmd O |
| Go to file / `navigation.file` | Ctrl Shift N | Cmd Shift O | Ctrl P | Cmd P |
| Save file / `file.save` | — | — | Ctrl S | Cmd S |
| Save all files / `file.save_all` | Ctrl S | Cmd S | Ctrl Shift S | Cmd Shift S |
| Undo / `edit.undo` | Ctrl Z | Cmd Z | Ctrl Z | Cmd Z |
| Redo / `edit.redo` | Ctrl Shift Z | Cmd Shift Z | Ctrl Shift Z | Cmd Shift Z |
| Find in file / `search.find` | Ctrl F | Cmd F | Ctrl F | Cmd F |
| Search saved project files / `search.project` | Ctrl Shift F | Cmd Shift F | Ctrl Shift F | Cmd Shift F |
| Cargo check / `cargo.check` | F5 (Kedr: Cargo check) | F5 (Kedr: Cargo check) | F5 (Kedr: Cargo check) | F5 (Kedr: Cargo check) |
| Cargo test / `cargo.test` | Ctrl Shift F5 (Kedr: Cargo test) | Cmd Shift F5 (Kedr: Cargo test) | Ctrl Shift F5 (Kedr: Cargo test) | Cmd Shift F5 (Kedr: Cargo test) |
| Close current tab / `tab.close` | Ctrl W | Cmd W | Ctrl W | Cmd W |
| Toggle sidebar / `view.sidebar` | — | — | — | — |
| Toggle output panel / `view.output` | — | — | — | — |
| Appearance & settings / `settings.appearance` | Ctrl Alt S | Cmd , | Ctrl , | Cmd , |
| Command palette / `commands.search` | Ctrl Shift A | Cmd Shift A | Ctrl Shift P | Cmd Shift P |

A dash means no preset chord, not an unavailable action: use the command palette. F5/Cargo test chords and Ctrl+O are Kedr assignments, not claims that their JetBrains action semantics are identical. Ctrl+Tab cycles editor tabs here, not a complete JetBrains switcher.

## Focus and editing

Undo/redo, select-all, duplicate and delete-line shortcuts require actual editor focus; they do not mutate the document while a search field, palette or modal owns focus. Clipboard, arrow/word movement, Home/End, selection extension, Backspace/Delete, Enter and Tab/Shift+Tab remain editor-native. These are Unicode-scalar operations, not complete grapheme/bidi or native IME acceptance.

## Custom shortcuts

Settings → Keyboard shortcuts → Edit lets you type or record a single combination, apply it, unbind it, or restore the preset. Reset this profile restores all overrides for the active platform/profile. Changes are persisted in local preferences and immediately drive the same dispatch/menu/palette hints. Conflicts are rejected atomically: no existing action is silently displaced. Copy/cut/paste, cancellation, normal typing/navigation and common OS chords are protected. If resetting one action conflicts with another override, unbind that other action first or reset the whole profile. Details: [custom shortcut contract](SHORTCUTS.md).

## Not implemented

Multi-stroke chords, double-modifier triggers, full Search Everywhere, semantic completion/refactoring, debugger, terminal, Git and product-specific shortcut families are not delivered. Unsupported features are not mapped to fake actions. See [parity gates](PARITY.md).

## Reference baseline

[Windows/Linux reference](https://www.jetbrains.com/help/idea/reference-keymap-win-default.html) · [macOS reference](https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html). Keymaps differ by product, version and customization. Compatible naming is not affiliation.

