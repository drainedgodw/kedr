use super::*;
impl Shell {
    pub(super) fn palette_ui(&mut self, ctx: &Context, p: Palette) {
        let Some(mut palette) = self.palette.take() else {
            return;
        };
        let (mut close, mut chosen, mut path) = (false, None, None);
        egui::Modal::new(Id::new("kedr-palette")).show(ctx, |ui| {
            ui.set_width(620.0_f32.min(ctx.content_rect().width() - 80.));
            ui.heading(if palette.files {
                "Go to file"
            } else {
                "Commands"
            });
            let r = ui.add(
                singleline(&mut palette.query)
                    .hint_text(if palette.files {
                        "Type a file name or path…"
                    } else {
                        "What would you like to do?"
                    })
                    .char_limit(256)
                    .desired_width(f32::INFINITY),
            );
            if palette.focus {
                r.request_focus();
                palette.focus = false;
            }
            if r.changed() {
                palette.selected = 0;
            }
            let files = if palette.files {
                self.workspace
                    .as_ref()
                    .map(|w| w.quick_files(&palette.query, 10))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let commands: Vec<_> = if palette.files {
                Vec::new()
            } else {
                Action::ALL
                    .into_iter()
                    .filter(|a| {
                        a.label()
                            .to_lowercase()
                            .contains(&palette.query.to_lowercase())
                    })
                    .collect()
            };
            let count = if palette.files {
                files.len()
            } else {
                commands.len()
            };
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                palette.selected = (palette.selected + 1).min(count.saturating_sub(1));
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                palette.selected = palette.selected.saturating_sub(1);
            }
            let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            if count == 0 {
                ui.label(if self.workspace.is_none() && palette.files {
                    "Open a workspace to browse its files."
                } else {
                    "No matching results."
                });
            }
            egui::ScrollArea::vertical()
                .max_height(440.)
                .show(ui, |ui| {
                    for i in 0..count {
                        let label = if palette.files {
                            files[i]
                                .strip_prefix(&self.workspace.as_ref().unwrap().project.root)
                                .unwrap()
                                .display()
                                .to_string()
                        } else {
                            format!("{}    {}", commands[i].label(), self.shortcut(commands[i]))
                        };
                        if ui
                            .add(
                                egui::Button::new(label)
                                    .selected(i == palette.selected)
                                    .wrap_mode(egui::TextWrapMode::Truncate)
                                    .min_size(Vec2::new(ui.available_width(), 44.)),
                            )
                            .clicked()
                            || (enter && i == palette.selected)
                        {
                            if palette.files {
                                path = Some(files[i].clone());
                            } else {
                                chosen = Some(commands[i]);
                            }
                            close = true;
                        }
                    }
                });
            ui.add_space(8.);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Up / Down navigate · Enter open · Esc cancel").color(p.muted),
                );
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if !close {
            self.palette = Some(palette);
        }
        if let Some(a) = chosen {
            self.action(a);
        }
        if let Some(path) = path {
            self.pending_hit = None;
            self.effects.push(Effect::OpenFile(path));
        }
    }
    pub(super) fn dialog_ui(&mut self, ctx: &Context, p: Palette) {
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        let (mut close, mut discard, mut save) = (false, false, false);
        let mut next_dialog = None;
        let was_recording = self.record_shortcut && matches!(dialog, Dialog::EditShortcut(_));
        if was_recording {
            let pressed = ctx.input_mut(|i| {
                let pressed = i.events.iter().find_map(|e| match e {
                    egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        repeat: false,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                });
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key { pressed: true, .. } | egui::Event::Text(_)
                    )
                });
                pressed
            });
            if let Some((key, modifiers)) = pressed {
                self.record_shortcut = false;
                if key != Key::Escape {
                    match crate::Chord::from_event(key, modifiers, cfg!(target_os = "macos")) {
                        Ok(chord) => {
                            self.shortcut_draft = chord.label(cfg!(target_os = "macos"));
                            self.shortcut_error = None;
                        }
                        Err(error) => self.shortcut_error = Some(error),
                    }
                }
            }
        }
        egui::Modal::new(Id::new("kedr-dialog")).show(ctx, |ui| {
            ui.set_width(520.0_f32.min(ctx.content_rect().width() - 80.));
            match &dialog {
                Dialog::Keymap => {
                    ui.heading("Keyboard shortcuts");
                    ui.label(RichText::new("Working actions only. Unsupported IDE features are not fake shortcuts.").color(p.muted));
                    ui.horizontal_wrapped(|ui| {
                        for profile in [crate::Keymap::JetBrains, crate::Keymap::Kedr] {
                            if ui.selectable_label(self.settings.keymap == profile, profile.name()).clicked() {
                                self.settings.keymap = profile;
                                self.refresh_shortcuts();
                                self.effects.push(Effect::PreferencesChanged);
                            }
                        }
                    });
                    ui.add(singleline(&mut self.shortcut_filter).hint_text("Search actions, IDs or shortcuts…").char_limit(256).desired_width(f32::INFINITY));
                    let filter = self.shortcut_filter.to_lowercase();
                    egui::ScrollArea::vertical().max_height((ctx.content_rect().height() - 360.).clamp(120., 360.)).show(ui, |ui| {
                        for action in Action::ALL {
                            let hint = self.shortcut(action);
                            if !format!("{} {} {}", action.label(), action.id(), hint).to_lowercase().contains(&filter) { continue; }
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(action.label()).strong().color(p.text)).on_hover_text(action.id());
                                    ui.label(RichText::new(&hint).color(p.accent));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.add(egui::Button::new("Edit").min_size(Vec2::new(64.,44.))).clicked() {
                                        let id = crate::override_id(self.settings.keymap, cfg!(target_os = "macos"), action);
                                        self.shortcut_draft = self.settings.shortcuts.get(&id).cloned().unwrap_or_else(|| {
                                            crate::bindings(self.settings.keymap, cfg!(target_os = "macos")).into_iter().find(|b| b.action == action).map_or_else(String::new, |b| crate::Chord {key:b.key,modifiers:b.modifiers}.label(cfg!(target_os = "macos")))
                                        });
                                        self.shortcut_error = None;
                                        self.record_shortcut = false;
                                        next_dialog = Some(Dialog::EditShortcut(action)); close = true;
                                    }
                                });
                            });
                            ui.add_space(4.);
                            ui.separator();
                        }
                    });
                    ui.label(RichText::new("Edit or record a single shortcut. Conflicts are rejected; typing and clipboard keys stay protected. Full IDE parity is still planned.").color(p.muted));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Done").clicked() { close = true; self.editor_cache.focus_next = true; }
                        if ui.button("Reset this profile").clicked() {
                            let prefix = format!("{}:{}:", self.settings.keymap.id(), if cfg!(target_os = "macos") {"mac"} else {"other"});
                            self.settings.shortcuts.retain(|id,_| !id.starts_with(&prefix));
                            self.refresh_shortcuts();
                            self.effects.push(Effect::PreferencesChanged);
                        }
                    });
                }
                Dialog::EditShortcut(action) => {
                    ui.heading("Edit shortcut");
                    ui.label(RichText::new(action.label()).strong());
                    ui.label(RichText::new(format!("{} · {}", self.settings.keymap.name(), action.id())).color(p.muted));
                    ui.add_space(12.);
                    ui.label(if cfg!(target_os = "macos") {"Example: Cmd+Alt+L or F8"} else {"Example: Ctrl+Alt+L or F8"});
                    ui.add_enabled(!self.record_shortcut, singleline(&mut self.shortcut_draft).hint_text("No shortcut").char_limit(96).desired_width(f32::INFINITY));
                    if ui.add(egui::Button::new(if self.record_shortcut {"Press keys… Esc cancels recording"} else {"Record keys"}).min_size(Vec2::new(200.,44.))).clicked() {
                        self.record_shortcut = !self.record_shortcut;
                    }
                    ui.add_space(8.);
                    ui.label(RichText::new("One combination per action. Changes affect only this platform and profile. OS-reserved shortcuts may not reach this window.").color(p.muted));
                    if let Some(error) = &self.shortcut_error { ui.label(RichText::new(error).color(p.amber)); }
                    ui.add_space(12.);
                    ui.horizontal_wrapped(|ui| {
                        let apply = ui.add_enabled(!self.record_shortcut, egui::Button::new("Apply")).clicked();
                        let unbind = ui.add_enabled(!self.record_shortcut, egui::Button::new("Unbind")).clicked();
                        let reset = ui.add_enabled(!self.record_shortcut, egui::Button::new("Use preset")).clicked();
                        if apply || unbind || reset {
                            let draft = self.shortcut_draft.clone();
                            let value = if reset { None } else if unbind { Some("") } else { Some(draft.as_str()) };
                            match self.remap_shortcut(*action, value) {
                                Ok(()) => { next_dialog = Some(Dialog::Keymap); close = true; }
                                Err(error) => self.shortcut_error = Some(error),
                            }
                        }
                        if ui.button("Cancel").clicked() { next_dialog = Some(Dialog::Keymap); close = true; self.record_shortcut = false; }
                    });
                }
                Dialog::GoToLine => {
                    ui.heading("Go to line / column");
                    ui.label("Enter a 1-based line or line:column. Columns use Unicode scalars.");
                    let response = ui.add(singleline(&mut self.goto).hint_text("12:4").char_limit(32).desired_width(f32::INFINITY));
                    if self.goto_focus { response.request_focus(); self.goto_focus = false; }
                    let enter = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Go").min_size(Vec2::splat(44.))).clicked() || enter {
                            let parts: Vec<_> = self.goto.trim().split(':').collect();
                            let result = if parts.len() <= 2 {
                                parts[0].parse::<usize>().ok().zip(if parts.len() == 2 { parts[1].parse().ok() } else { Some(1) })
                            } else { None };
                            if let Some((row, col)) = result {
                                let result = self.workspace.as_mut().and_then(Workspace::active_mut).ok_or_else(|| "Open a file first".to_string()).and_then(|t| t.goto(row, col));
                                match result { Ok(()) => { close = true; self.editor_cache.focus_next = true; }, Err(e) => self.error(e) }
                            } else { self.error("Enter a valid line or line:column"); }
                        }
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                }
                Dialog::Settings => {
                    ui.heading("Make it yours");
                    ui.label(RichText::new("One visual language across every Kedr product.").color(p.muted));
                    ui.add_space(16.);
                    ui.label("Appearance");
                    ui.horizontal(|ui| {
                        for (theme, label) in [(Theme::Dark, "Midnight"), (Theme::Light, "Daylight")] {
                            if ui.add(egui::Button::new(label)
                                .selected(self.settings.theme == theme)
                                .min_size(Vec2::new(170., 44.))).clicked() {
                                self.settings.theme = theme;
                                apply_theme(ctx, theme);
                                self.effects.push(Effect::PreferencesChanged);
                            }
                        }
                    });
                    ui.add_space(12.);
                    ui.label("Code font size");
                    if ui.add(egui::Slider::new(&mut self.settings.font_size, 14.0..=24.0)
                        .suffix(" px")).changed() {
                        self.effects.push(Effect::PreferencesChanged);
                    }
                    ui.add_space(16.);
                    ui.label(format!("Keyboard profile: {}", self.settings.keymap.name()));
                    if ui.button("Keyboard shortcuts").clicked() {
                        next_dialog = Some(Dialog::Keymap); close = true;
                    }
                    ui.add_space(20.);
                    ui.label(RichText::new("Code navigation uses Unicode scalars, not grapheme clusters.\nBasic syntax colors are not semantic language analysis.").color(p.muted));
                    if ui.button("Done").clicked() {
                        close = true;
                        self.editor_cache.focus_next = true;
                    }
                }
                Dialog::Trust(kind) => {
                    ui.heading("Allow project code to run?");
                    ui.label(format!("{} can execute this project's build scripts and dependencies.", kind.label()));
                    if let Some(w) = &self.workspace {
                        ui.label(RichText::new(w.project.root.display().to_string()).monospace().color(p.amber));
                    }
                    ui.label("Nothing is executed automatically. Only continue if you trust this workspace.");
                    ui.add_space(16.);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Run once").clicked() {
                            self.effects.push(Effect::RunTask(*kind));
                            close = true;
                        }
                        if ui.button("Trust this session & run").clicked() {
                            self.trusted = true;
                            self.effects.push(Effect::RunTask(*kind));
                            close = true;
                        }
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                }
                _ => {
                    ui.heading("Keep your changes?");
                    ui.label("Some files have unsaved edits. Saving can fail if a file changed outside Kedr; that will not silently overwrite it.");
                    ui.add_space(16.);
                    ui.horizontal_wrapped(|ui| {
                        save = ui.button("Save & continue").clicked();
                        discard = ui.button("Discard changes").clicked();
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                }
            }
        });
        if save {
            let result = match &dialog {
                Dialog::Close(i) => self
                    .workspace
                    .as_mut()
                    .and_then(|w| w.tabs.get_mut(*i))
                    .map_or(Ok(()), |t| t.save()),
                _ => self.workspace.as_mut().map_or(Ok(()), |w| w.save_all()),
            };
            match result {
                Ok(()) => discard = true,
                Err(e) => self.error(e.to_string()),
            }
        }
        if discard {
            match dialog.clone() {
                Dialog::Close(i) => {
                    if let Some(w) = &mut self.workspace {
                        w.discard(i)
                    }
                }
                Dialog::Switch(path) => self.effects.push(Effect::OpenProject(path)),
                Dialog::Quit => self.effects.push(Effect::Quit),
                _ => {}
            }
            close = true;
        }
        if !was_recording && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            close = true;
            self.record_shortcut = false;
            if matches!(dialog, Dialog::EditShortcut(_)) {
                next_dialog = Some(Dialog::Keymap);
            }
        }
        if close {
            self.dialog = next_dialog;
        }
    }
    /// Deterministic capture states; no commands or project files are modified on disk.
    pub fn capture_state(&mut self, state: &str) {
        match state {
            "palette" => self.action(Action::QuickOpen),
            "commands" => self.action(Action::Commands),
            "settings" => self.action(Action::Settings),
            "keymap" => self.action(Action::Keybindings),
            "remap" | "remap-conflict" | "remap-record" => {
                self.dialog = Some(Dialog::EditShortcut(Action::QuickOpen));
                self.shortcut_draft = if state == "remap-conflict" {
                    "Ctrl+S"
                } else {
                    "Ctrl+Alt+L"
                }
                .into();
                self.shortcut_error = if state == "remap-conflict" {
                    Some("Go to file conflicts with Save all files (Ctrl+S)".into())
                } else {
                    None
                };
                self.record_shortcut = state == "remap-record";
            }
            "problems" | "output" => {
                self.settings.show_output = true;
                self.problems_view = state == "problems";
                self.task = Some(TaskRunner::capture_fixture());
            }
            "goto" => self.action(Action::GoToLine),
            "find" => self.action(Action::Replace),
            "trust" => self.action(Action::Check),
            "search" => {
                self.action(Action::Search);
                self.search = "fn".into();
                self.effects.push(Effect::Search(self.search.clone()));
                self.search_loading = true;
            }
            "unsaved" => {
                if let Some(t) = self.workspace.as_mut().and_then(Workspace::active_mut) {
                    let _ = t.insert("// unsaved preview\n");
                }
                self.action(Action::CloseTab);
            }
            "error" => self.error(
                "File changed externally; refusing overwrite. Your edits remain in the editor.",
            ),
            _ => {}
        }
    }
}

#[cfg(test)]
mod remap_tests {
    use super::*;
    fn frame(s: &mut Shell, ctx: &Context, events: Vec<egui::Event>) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(900., 640.),
                )),
                events,
                focused: true,
                ..Default::default()
            },
            |ctx| {
                s.show(ctx);
            },
        );
    }
    fn press(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn recorder_captures_a_chord_without_applying_or_closing() {
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.capture_state("remap-record");
        let ctx = Context::default();
        frame(&mut s, &ctx, vec![press(Key::F8)]);
        assert_eq!(s.shortcut_draft, "F8");
        assert!(!s.record_shortcut);
        assert!(s.settings.shortcuts.is_empty());
        assert!(matches!(
            s.dialog,
            Some(Dialog::EditShortcut(Action::QuickOpen))
        ));
    }
    #[test]
    fn escape_cancels_recording_before_it_cancels_dialog() {
        let mut s = Shell::new("CodeCedar", Settings::default());
        s.capture_state("remap-record");
        let ctx = Context::default();
        frame(&mut s, &ctx, vec![press(Key::Escape)]);
        assert!(!s.record_shortcut);
        assert!(matches!(s.dialog, Some(Dialog::EditShortcut(_))));
        frame(&mut s, &ctx, vec![press(Key::Escape)]);
        assert!(matches!(s.dialog, Some(Dialog::Keymap)));
    }
}
