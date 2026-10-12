//! Generate the public shortcut table from the actual dispatch registry.
use kedr_ui::{Action, Keymap, hint};
fn main() {
    println!("# Implemented keyboard profiles\n");
    println!(
        "Generated with `cargo run --locked -p kedr-ui --example keymap`. The same registry drives dispatch, menu/palette hints and the in-app Keyboard shortcuts screen.\n"
    );
    println!(
        "The default profile is a **JetBrains-compatible subset**, not full keymap or feature parity. Kedr classic is selectable in Settings → Keyboard shortcuts; the choice is persisted locally.\n"
    );
    println!(
        "| Working action / ID | Compatible Windows/Linux | Compatible macOS | Classic Windows/Linux | Classic macOS |"
    );
    println!("| --- | --- | --- | --- | --- |");
    for a in Action::ALL {
        println!(
            "| {} / `{}` | {} | {} | {} | {} |",
            a.label(),
            a.id(),
            hint(Keymap::JetBrains, a, false),
            hint(Keymap::JetBrains, a, true),
            hint(Keymap::Kedr, a, false),
            hint(Keymap::Kedr, a, true)
        );
    }
    println!(
        "\nA dash means no preset chord, not an unavailable action: use the command palette. F5/Cargo test chords and Ctrl+O are Kedr assignments, not claims that their JetBrains action semantics are identical. Ctrl+Tab cycles editor tabs here, not a complete JetBrains switcher.\n"
    );
    println!(
        "## Focus and editing\n\nUndo/redo, select-all, duplicate and delete-line shortcuts require actual editor focus; they do not mutate the document while a search field, palette or modal owns focus. Clipboard, arrow/word movement, Home/End, selection extension, Backspace/Delete, Enter and Tab/Shift+Tab remain editor-native. These are Unicode-scalar operations, not complete grapheme/bidi or native IME acceptance.\n"
    );
    println!(
        "## Not implemented\n\nCustom remapping, conflict editing, multi-stroke chords, double-modifier triggers, full Search Everywhere, semantic completion/refactoring, debugger, terminal, Git and product-specific shortcut families are not delivered. Unsupported features are not mapped to fake actions. See [parity gates](PARITY.md).\n"
    );
    println!(
        "## Reference baseline\n\n[Windows/Linux reference](https://www.jetbrains.com/help/idea/reference-keymap-win-default.html) · [macOS reference](https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html). Keymaps differ by product, version and customization. Compatible naming is not affiliation.\n"
    );
}
