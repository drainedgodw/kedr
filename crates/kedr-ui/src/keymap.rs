//! Discoverable, platform-aware bindings for implemented actions only.
use crate::Action;
use egui::{Key, Modifiers};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Keymap {
    #[default]
    JetBrains,
    Kedr,
}
impl Keymap {
    pub fn name(self) -> &'static str {
        match self {
            Self::JetBrains => "JetBrains-compatible",
            Self::Kedr => "Kedr classic",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::JetBrains => "jetbrains",
            Self::Kedr => "kedr",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub action: Action,
    pub modifiers: Modifiers,
    pub key: Key,
    pub hint: &'static str,
    pub editor_only: bool,
}
pub(crate) fn canonical_modifiers(mut m: Modifiers, mac: bool) -> Modifiers {
    if mac {
        m.command |= m.mac_cmd;
        m.mac_cmd = false;
    } else {
        m.command |= m.ctrl;
        m.ctrl = false;
        m.mac_cmd = false;
    }
    m
}
impl Binding {
    pub(crate) fn matches_key(self, key: Key, modifiers: Modifiers, mac: bool) -> bool {
        self.key == key
            && canonical_modifiers(modifiers, mac) == canonical_modifiers(self.modifiers, mac)
    }
    pub(crate) fn consume(self, input: &mut egui::InputState) -> bool {
        let mut consumed = false;
        input.events.retain(|event| {
            let matches = matches!(event, egui::Event::Key {
                key, modifiers, pressed: true, ..
            } if self.matches_key(*key, *modifiers, cfg!(target_os = "macos")));
            consumed |= matches;
            !matches
        });
        consumed
    }
}
pub fn bindings(profile: Keymap, mac: bool) -> Vec<Binding> {
    use Action::*;
    let c = Modifiers::COMMAND;
    let s = Modifiers::SHIFT;
    let a = Modifiers::ALT;
    let none = Modifiers::NONE;
    let mut rows = vec![
        (
            OpenProject,
            c,
            Key::O,
            if mac { "Cmd O" } else { "Ctrl O" },
            false,
        ),
        (Find, c, Key::F, if mac { "Cmd F" } else { "Ctrl F" }, false),
        (
            Replace,
            c,
            Key::R,
            if mac { "Cmd R" } else { "Ctrl R" },
            false,
        ),
        (
            Search,
            c | s,
            Key::F,
            if mac { "Cmd Shift F" } else { "Ctrl Shift F" },
            false,
        ),
        (
            CloseTab,
            c,
            Key::W,
            if mac { "Cmd W" } else { "Ctrl W" },
            false,
        ),
        (Undo, c, Key::Z, if mac { "Cmd Z" } else { "Ctrl Z" }, true),
        (
            Redo,
            c | s,
            Key::Z,
            if mac { "Cmd Shift Z" } else { "Ctrl Shift Z" },
            true,
        ),
        (
            SelectAll,
            c,
            Key::A,
            if mac { "Cmd A" } else { "Ctrl A" },
            true,
        ),
        (Check, none, Key::F5, "F5 (Kedr: Cargo check)", false),
        (
            Test,
            c | s,
            Key::F5,
            if mac {
                "Cmd Shift F5 (Kedr: Cargo test)"
            } else {
                "Ctrl Shift F5 (Kedr: Cargo test)"
            },
            false,
        ),
        (NextTab, Modifiers::CTRL, Key::Tab, "Ctrl Tab", false),
        (
            PreviousTab,
            Modifiers::CTRL | s,
            Key::Tab,
            "Ctrl Shift Tab",
            false,
        ),
    ];
    match profile {
        Keymap::JetBrains => rows.extend([
            (
                QuickOpen,
                c | s,
                if mac { Key::O } else { Key::N },
                if mac { "Cmd Shift O" } else { "Ctrl Shift N" },
                false,
            ),
            (
                Commands,
                c | s,
                Key::A,
                if mac { "Cmd Shift A" } else { "Ctrl Shift A" },
                false,
            ),
            (
                SaveAll,
                c,
                Key::S,
                if mac { "Cmd S" } else { "Ctrl S" },
                false,
            ),
            (
                Settings,
                if mac { c } else { c | a },
                if mac { Key::Comma } else { Key::S },
                if mac { "Cmd ," } else { "Ctrl Alt S" },
                false,
            ),
            (
                Project,
                if mac { c } else { a },
                Key::Num1,
                if mac { "Cmd 1" } else { "Alt 1" },
                false,
            ),
            (
                GoToLine,
                c,
                if mac { Key::L } else { Key::G },
                if mac { "Cmd L" } else { "Ctrl G" },
                false,
            ),
            (
                FindNext,
                if mac { c } else { none },
                if mac { Key::G } else { Key::F3 },
                if mac { "Cmd G" } else { "F3" },
                false,
            ),
            (
                FindPrevious,
                if mac { c | s } else { s },
                if mac { Key::G } else { Key::F3 },
                if mac { "Cmd Shift G" } else { "Shift F3" },
                false,
            ),
            (
                Duplicate,
                c,
                Key::D,
                if mac { "Cmd D" } else { "Ctrl D" },
                true,
            ),
            (
                DeleteLine,
                c,
                if mac { Key::Backspace } else { Key::Y },
                if mac { "Cmd Backspace" } else { "Ctrl Y" },
                true,
            ),
        ]),
        Keymap::Kedr => rows.extend([
            (
                QuickOpen,
                c,
                Key::P,
                if mac { "Cmd P" } else { "Ctrl P" },
                false,
            ),
            (
                Commands,
                c | s,
                Key::P,
                if mac { "Cmd Shift P" } else { "Ctrl Shift P" },
                false,
            ),
            (Save, c, Key::S, if mac { "Cmd S" } else { "Ctrl S" }, false),
            (
                SaveAll,
                c | s,
                Key::S,
                if mac { "Cmd Shift S" } else { "Ctrl Shift S" },
                false,
            ),
            (
                Settings,
                c,
                Key::Comma,
                if mac { "Cmd ," } else { "Ctrl ," },
                false,
            ),
            (
                Project,
                c,
                Key::Num1,
                if mac { "Cmd 1" } else { "Ctrl 1" },
                false,
            ),
            (
                GoToLine,
                c,
                Key::G,
                if mac { "Cmd G" } else { "Ctrl G" },
                false,
            ),
            (FindNext, none, Key::F3, "F3", false),
            (FindPrevious, s, Key::F3, "Shift F3", false),
            (
                Duplicate,
                c | s,
                Key::D,
                if mac { "Cmd Shift D" } else { "Ctrl Shift D" },
                true,
            ),
        ]),
    }
    rows.into_iter()
        .map(|(action, modifiers, key, hint, editor_only)| Binding {
            action,
            modifiers,
            key,
            hint,
            editor_only,
        })
        .collect()
}
pub fn hint(profile: Keymap, action: Action, mac: bool) -> &'static str {
    bindings(profile, mac)
        .into_iter()
        .find(|b| b.action == action)
        .map_or("—", |b| b.hint)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_profile_has_unique_chords_and_working_actions() {
        for profile in [Keymap::JetBrains, Keymap::Kedr] {
            for mac in [false, true] {
                let rows = bindings(profile, mac);
                for (i, row) in rows.iter().enumerate() {
                    assert!(Action::ALL.contains(&row.action));
                    assert!(
                        !rows[..i]
                            .iter()
                            .any(|r| r.key == row.key && r.modifiers == row.modifiers)
                    );
                }
            }
        }
    }
    #[test]
    fn exact_dispatch_never_shadows_a_more_specific_chord() {
        for profile in [Keymap::JetBrains, Keymap::Kedr] {
            for mac in [false, true] {
                let rows = bindings(profile, mac);
                for target in &rows {
                    let mut physical = target.modifiers;
                    if physical.command {
                        if mac {
                            physical.mac_cmd = true;
                        } else {
                            physical.ctrl = true;
                        }
                    }
                    for modifiers in [target.modifiers, physical] {
                        let matched: Vec<_> = rows
                            .iter()
                            .filter(|b| b.matches_key(target.key, modifiers, mac))
                            .map(|b| b.action)
                            .collect();
                        assert_eq!(
                            matched,
                            vec![target.action],
                            "{} {}",
                            profile.name(),
                            target.hint
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn jetbrains_save_and_destructive_chords_are_not_mislabeled() {
        let rows = bindings(Keymap::JetBrains, false);
        assert_eq!(
            rows.iter()
                .find(|b| b.key == Key::S && b.modifiers == Modifiers::COMMAND)
                .unwrap()
                .action,
            Action::SaveAll
        );
        let delete = rows.iter().find(|b| b.key == Key::Y).unwrap();
        assert_eq!(delete.action, Action::DeleteLine);
        assert!(delete.editor_only);
        assert_eq!(
            hint(Keymap::JetBrains, Action::QuickOpen, true),
            "Cmd Shift O"
        );
    }
}
