//! Persistable single-stroke overrides. Validation is atomic and platform scoped.
use crate::{Action, Binding, Keymap, bindings};
use egui::{Key, Modifiers};
use std::collections::BTreeMap;
pub type ShortcutOverrides = BTreeMap<String, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: Modifiers,
    pub key: Key,
}
impl Chord {
    pub fn parse(text: &str, mac: bool) -> Result<Self, String> {
        if text.len() > 96 {
            return Err("Shortcut is too long".into());
        }
        let parts: Vec<_> = text.split('+').map(str::trim).collect();
        if parts.is_empty() || parts.iter().any(|p| p.is_empty()) {
            return Err("Use one key, for example Ctrl+Alt+L or F8".into());
        }
        let mut modifiers = Modifiers::NONE;
        for token in &parts[..parts.len() - 1] {
            let modifier = match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => {
                    if mac {
                        Modifiers::CTRL
                    } else {
                        Modifiers::COMMAND
                    }
                }
                "cmd" | "command" if mac => Modifiers::COMMAND,
                "shift" => Modifiers::SHIFT,
                "alt" | "option" => Modifiers::ALT,
                _ => return Err(format!("Unknown modifier: {token}")),
            };
            if modifiers == (modifiers | modifier) {
                return Err(format!("Repeated modifier: {token}"));
            }
            modifiers |= modifier;
        }
        let name = parts[parts.len() - 1];
        let key = Key::from_name(name)
            .or_else(|| {
                Key::ALL
                    .iter()
                    .copied()
                    .find(|k| k.name().eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| format!("Unknown key: {name}. Use Plus for the + key."))?;
        let chord = Self { modifiers, key };
        chord.validate(mac)?;
        Ok(chord)
    }
    pub fn from_event(key: Key, mut modifiers: Modifiers, mac: bool) -> Result<Self, String> {
        if mac {
            modifiers.command |= modifiers.mac_cmd;
            modifiers.mac_cmd = false;
        } else {
            modifiers.command |= modifiers.ctrl;
            modifiers.ctrl = false;
        }
        let chord = Self { key, modifiers };
        chord.validate(mac)?;
        Ok(chord)
    }
    fn validate(self, mac: bool) -> Result<(), String> {
        let m = self.modifiers;
        let function = self
            .key
            .name()
            .strip_prefix('F')
            .is_some_and(|v| v.parse::<u8>().is_ok_and(|n| (1..=35).contains(&n)));
        if self.key == Key::Escape || matches!(self.key, Key::Copy | Key::Cut | Key::Paste) {
            return Err("This key is reserved for cancellation or clipboard input".into());
        }
        if !(function || m.command || m.ctrl || m.alt) {
            return Err(
                "Use Ctrl, Cmd or Alt so normal typing and navigation stay available".into(),
            );
        }
        if m.command && !m.shift && !m.alt && matches!(self.key, Key::C | Key::X | Key::V) {
            return Err("Copy, cut and paste are reserved editor shortcuts".into());
        }
        if (mac && m.command && matches!(self.key, Key::Q | Key::Tab))
            || (!mac && m.alt && self.key == Key::F4)
            || (!mac && m.command && m.alt && self.key == Key::Delete)
        {
            return Err("This shortcut is reserved by the operating system".into());
        }
        Ok(())
    }
    pub fn label(self, mac: bool) -> String {
        let mut parts = Vec::new();
        if self.modifiers.ctrl {
            parts.push("Ctrl");
        }
        if self.modifiers.command {
            parts.push(if mac { "Cmd" } else { "Ctrl" });
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }
        parts.push(self.key.name());
        parts.join("+")
    }
}
pub fn override_id(profile: Keymap, mac: bool, action: Action) -> String {
    format!(
        "{}:{}:{}",
        profile.id(),
        if mac { "mac" } else { "other" },
        action.id()
    )
}
fn editor_only(action: Action) -> bool {
    matches!(
        action,
        Action::Undo | Action::Redo | Action::SelectAll | Action::Duplicate | Action::DeleteLine
    )
}
pub fn resolved_bindings(
    profile: Keymap,
    mac: bool,
    overrides: &ShortcutOverrides,
) -> Result<Vec<Binding>, String> {
    let mut rows = bindings(profile, mac);
    for action in Action::ALL {
        let Some(text) = overrides.get(&override_id(profile, mac, action)) else {
            continue;
        };
        rows.retain(|b| b.action != action);
        if text.is_empty() {
            continue;
        }
        let chord = Chord::parse(text, mac)?;
        rows.push(Binding {
            action,
            key: chord.key,
            modifiers: chord.modifiers,
            editor_only: editor_only(action),
            hint: "",
        });
    }
    for (i, row) in rows.iter().enumerate() {
        if let Some(other) = rows[..i]
            .iter()
            .find(|b| b.matches_key(row.key, row.modifiers, mac))
        {
            return Err(format!(
                "{} conflicts with {} ({})",
                row.action.label(),
                other.action.label(),
                Chord {
                    key: row.key,
                    modifiers: row.modifiers
                }
                .label(mac)
            ));
        }
    }
    Ok(rows)
}
pub fn validate_overrides(overrides: &ShortcutOverrides) -> Result<(), String> {
    if overrides.len() > Action::ALL.len() * 4 {
        return Err("Too many shortcut overrides".into());
    }
    for id in overrides.keys() {
        if ![Keymap::JetBrains, Keymap::Kedr].into_iter().any(|p| {
            [false, true]
                .into_iter()
                .any(|m| Action::ALL.into_iter().any(|a| override_id(p, m, a) == *id))
        }) {
            return Err(format!("Unknown shortcut action/profile: {id}"));
        }
    }
    for profile in [Keymap::JetBrains, Keymap::Kedr] {
        for mac in [false, true] {
            resolved_bindings(profile, mac, overrides)?;
        }
    }
    Ok(())
}
/// None resets to the preset; an empty string explicitly unbinds the action.
pub fn set_override(
    overrides: &mut ShortcutOverrides,
    profile: Keymap,
    mac: bool,
    action: Action,
    value: Option<&str>,
) -> Result<(), String> {
    let mut candidate = overrides.clone();
    let id = override_id(profile, mac, action);
    match value {
        None => {
            candidate.remove(&id);
        }
        Some("") => {
            candidate.insert(id, String::new());
        }
        Some(text) => {
            candidate.insert(id, Chord::parse(text, mac)?.label(mac));
        }
    }
    validate_overrides(&candidate)?;
    *overrides = candidate;
    Ok(())
}
pub fn resolved_hint(
    profile: Keymap,
    mac: bool,
    action: Action,
    overrides: &ShortcutOverrides,
) -> String {
    resolved_bindings(profile, mac, overrides)
        .unwrap_or_else(|_| bindings(profile, mac))
        .into_iter()
        .find(|b| b.action == action)
        .map_or_else(
            || "—".into(),
            |b| {
                if b.hint.is_empty() {
                    Chord {
                        key: b.key,
                        modifiers: b.modifiers,
                    }
                    .label(mac)
                } else {
                    b.hint.into()
                }
            },
        )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_round_trips_both_platforms_and_rejects_unsafe_input() {
        for mac in [false, true] {
            for text in ["Ctrl+Alt+L", "F8", "Alt+Shift+F9", "Ctrl+Plus"] {
                let c = Chord::parse(text, mac).unwrap();
                assert_eq!(Chord::parse(&c.label(mac), mac).unwrap(), c);
            }
            for text in [
                "",
                "L",
                "Shift+L",
                "Escape",
                "Ctrl+Ctrl+L",
                "Ctrl++L",
                "Ctrl+L, Ctrl+K",
                "Win+L",
            ] {
                assert!(Chord::parse(text, mac).is_err(), "{text}");
            }
        }
        assert!(Chord::parse("Cmd+Q", true).is_err());
        assert!(Chord::parse("Alt+F4", false).is_err());
        assert!(Chord::parse("Ctrl+C", false).is_err());
        assert!(Chord::parse("Cmd+V", true).is_err());
    }
    #[test]
    fn conflicts_and_invalid_imports_never_mutate_config() {
        let mut m = ShortcutOverrides::new();
        set_override(
            &mut m,
            Keymap::JetBrains,
            false,
            Action::QuickOpen,
            Some("Ctrl+Alt+L"),
        )
        .unwrap();
        let old = m.clone();
        assert!(
            set_override(
                &mut m,
                Keymap::JetBrains,
                false,
                Action::Commands,
                Some("Ctrl+Alt+L")
            )
            .is_err()
        );
        assert_eq!(m, old);
        assert!(
            set_override(
                &mut m,
                Keymap::JetBrains,
                false,
                Action::QuickOpen,
                Some("Ctrl+S")
            )
            .is_err()
        );
        assert_eq!(m, old);
        m.insert("fake:action".into(), "F8".into());
        assert!(validate_overrides(&m).is_err());
    }
    #[test]
    fn overrides_are_isolated_unbindable_and_resettable() {
        let mut m = ShortcutOverrides::new();
        set_override(
            &mut m,
            Keymap::JetBrains,
            false,
            Action::QuickOpen,
            Some("Ctrl+Alt+L"),
        )
        .unwrap();
        assert_eq!(
            resolved_hint(Keymap::JetBrains, false, Action::QuickOpen, &m),
            "Ctrl+Alt+L"
        );
        assert_eq!(
            resolved_hint(Keymap::JetBrains, true, Action::QuickOpen, &m),
            "Cmd Shift O"
        );
        assert_eq!(
            resolved_hint(Keymap::Kedr, false, Action::QuickOpen, &m),
            "Ctrl P"
        );
        set_override(
            &mut m,
            Keymap::JetBrains,
            false,
            Action::QuickOpen,
            Some(""),
        )
        .unwrap();
        assert_eq!(
            resolved_hint(Keymap::JetBrains, false, Action::QuickOpen, &m),
            "—"
        );
        set_override(&mut m, Keymap::JetBrains, false, Action::QuickOpen, None).unwrap();
        assert!(m.is_empty());
    }
    #[test]
    fn action_without_preset_can_be_bound_and_context_is_preserved() {
        let mut m = ShortcutOverrides::new();
        set_override(
            &mut m,
            Keymap::JetBrains,
            false,
            Action::Keybindings,
            Some("F8"),
        )
        .unwrap();
        set_override(
            &mut m,
            Keymap::JetBrains,
            false,
            Action::Duplicate,
            Some("Ctrl+Alt+D"),
        )
        .unwrap();
        let rows = resolved_bindings(Keymap::JetBrains, false, &m).unwrap();
        assert!(
            !rows
                .iter()
                .find(|b| b.action == Action::Keybindings)
                .unwrap()
                .editor_only
        );
        assert!(
            rows.iter()
                .find(|b| b.action == Action::Duplicate)
                .unwrap()
                .editor_only
        );
    }
    #[test]
    fn recording_normalizes_physical_command_flags() {
        for mac in [false, true] {
            let modifiers = if mac {
                Modifiers::COMMAND | Modifiers::MAC_CMD | Modifiers::ALT
            } else {
                Modifiers::COMMAND | Modifiers::CTRL | Modifiers::ALT
            };
            let chord = Chord::from_event(Key::L, modifiers, mac).unwrap();
            assert_eq!(Chord::parse(&chord.label(mac), mac).unwrap(), chord);
        }
    }
}

#[cfg(test)]
mod strict_modifier_tests {
    use super::*;
    #[test]
    fn mac_control_and_command_do_not_shadow_each_other() {
        let command = Binding {
            action: Action::QuickOpen,
            key: Key::L,
            modifiers: Modifiers::COMMAND,
            hint: "",
            editor_only: false,
        };
        assert!(command.matches_key(Key::L, Modifiers::MAC_CMD | Modifiers::COMMAND, true));
        assert!(!command.matches_key(Key::L, Modifiers::COMMAND | Modifiers::CTRL, true));
        let control = Binding {
            modifiers: Modifiers::CTRL,
            ..command
        };
        assert!(!control.matches_key(Key::L, Modifiers::COMMAND | Modifiers::CTRL, true));
        assert!(control.matches_key(Key::L, Modifiers::CTRL, true));
        assert!(command.matches_key(Key::L, Modifiers::COMMAND | Modifiers::CTRL, false));
    }
    #[test]
    fn conflicting_preset_reset_is_atomic() {
        let mut overrides = ShortcutOverrides::new();
        set_override(
            &mut overrides,
            Keymap::JetBrains,
            false,
            Action::QuickOpen,
            Some("F8"),
        )
        .unwrap();
        set_override(
            &mut overrides,
            Keymap::JetBrains,
            false,
            Action::Keybindings,
            Some("Ctrl+Shift+N"),
        )
        .unwrap();
        let old = overrides.clone();
        assert!(
            set_override(
                &mut overrides,
                Keymap::JetBrains,
                false,
                Action::QuickOpen,
                None
            )
            .is_err()
        );
        assert_eq!(overrides, old);
    }
}
