use super::*;
impl Shell {
    pub(super) fn header(&mut self, ctx: &Context, p: Palette) {
        egui::TopBottomPanel::top("kedr-header")
            .exact_height(44.)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(6))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    wordmark(ui, p);
                    ui.add_space(6.);
                    for (title, actions) in [
                        (
                            "File",
                            &[
                                Action::OpenProject,
                                Action::Save,
                                Action::SaveAll,
                                Action::CloseTab,
                            ][..],
                        ),
                        (
                            "Edit",
                            &[
                                Action::Undo,
                                Action::Redo,
                                Action::SelectAll,
                                Action::Duplicate,
                                Action::DeleteLine,
                                Action::ToggleComment,
                                Action::MoveLineUp,
                                Action::MoveLineDown,
                                Action::JoinLines,
                            ][..],
                        ),
                        (
                            "Navigate",
                            &[
                                Action::QuickOpen,
                                Action::GoToLine,
                                Action::NextTab,
                                Action::PreviousTab,
                            ][..],
                        ),
                        (
                            "Search",
                            &[
                                Action::Find,
                                Action::Replace,
                                Action::FindNext,
                                Action::FindPrevious,
                                Action::Search,
                            ][..],
                        ),
                        (
                            "View",
                            &[
                                Action::Project,
                                Action::Sidebar,
                                Action::Output,
                                Action::Settings,
                                Action::Keybindings,
                                Action::Commands,
                            ][..],
                        ),
                        ("Run", &[Action::Check, Action::Test][..]),
                    ] {
                        ui.menu_button(title, |ui| {
                            for &action in actions {
                                let needs_editor = matches!(
                                    action,
                                    Action::Save
                                        | Action::CloseTab
                                        | Action::Undo
                                        | Action::Redo
                                        | Action::SelectAll
                                        | Action::Duplicate
                                        | Action::DeleteLine
                                        | Action::ToggleComment
                                        | Action::MoveLineUp
                                        | Action::MoveLineDown
                                        | Action::JoinLines
                                );
                                let enabled = !needs_editor
                                    || self
                                        .workspace
                                        .as_ref()
                                        .and_then(Workspace::active)
                                        .is_some();
                                if ui
                                    .add_enabled(
                                        enabled,
                                        egui::Button::new(format!(
                                            "{}    {}",
                                            action.label(),
                                            self.shortcut(action)
                                        )),
                                    )
                                    .clicked()
                                {
                                    self.action(action);
                                    ui.close();
                                    self.editor_cache.focus_next = needs_editor;
                                }
                            }
                        });
                    }
                    if ctx.content_rect().width() > 1100. {
                        ui.add_space(8.);
                        let name = self
                            .workspace
                            .as_ref()
                            .map_or("CodeCedar".into(), Workspace::name);
                        ui.label(
                            RichText::new(name.chars().take(20).collect::<String>()).color(p.muted),
                        )
                        .on_hover_text(name);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cargo_project = self.workspace.as_ref().is_some_and(|w| {
                            w.project.files.contains(&w.project.root.join("Cargo.toml"))
                        });
                        if ui
                            .add_enabled(
                                cargo_project,
                                egui::Button::new(RichText::new("Check").strong())
                                    .min_size(Vec2::new(70., 28.))
                                    .fill(p.accent_soft),
                            )
                            .on_hover_text(format!(
                                "Cargo check · {} · requires explicit trust",
                                self.shortcut(Action::Check)
                            ))
                            .clicked()
                        {
                            self.action(Action::Check);
                        }
                        let wide = ctx.content_rect().width() > 1150.;
                        let label = if wide {
                            format!("Go to file · {}", self.shortcut(Action::QuickOpen))
                        } else {
                            "Files".into()
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(label).color(p.muted))
                                    .min_size(Vec2::new(if wide { 230. } else { 80. }, 28.)),
                            )
                            .clicked()
                        {
                            self.action(Action::QuickOpen);
                        }
                    });
                });
            });
    }
    pub(super) fn status_bar(&mut self, ctx: &Context, p: Palette) {
        egui::TopBottomPanel::bottom("kedr-status")
            .exact_height(26.)
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(12, 1)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.0, p.green);
                    ui.label(RichText::new(&self.status).color(p.muted));
                    if let Some(tab) = self.workspace.as_ref().and_then(Workspace::active) {
                        let (line, col) = tab.position();
                        ui.separator();
                        ui.label(format!("Ln {}, Col {}", line + 1, col + 1));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(if tab.newline == "\r\n" { "CRLF" } else { "LF" });
                            ui.label("UTF-8");
                            ui.label(language(&tab.document.path));
                            if tab.cached_dirty {
                                ui.label(RichText::new("Modified").color(p.amber));
                            }
                        });
                    }
                });
            });
    }
    pub(super) fn activity(&mut self, ctx: &Context, p: Palette) {
        egui::SidePanel::left("kedr-activity")
            .exact_width(44.)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(6))
            .show(ctx, |ui| {
                for (kind, label, pane) in [
                    (Icon::Files, "Files", Pane::Files),
                    (Icon::Search, "Search saved project files", Pane::Search),
                ] {
                    let action = if pane == Pane::Files {
                        Action::Project
                    } else {
                        Action::Search
                    };
                    let shortcut = self.shortcut(action);
                    let tooltip = if shortcut == "—" {
                        label.to_string()
                    } else {
                        format!("{label} · {shortcut}")
                    };
                    if icon_button(
                        ui,
                        kind,
                        &tooltip,
                        self.settings.show_sidebar && self.pane == pane,
                        p,
                    )
                    .clicked()
                    {
                        if self.pane == pane {
                            self.settings.show_sidebar = !self.settings.show_sidebar;
                        } else {
                            self.settings.show_sidebar = true;
                            self.pane = pane;
                        }
                    }
                }
                if icon_button(
                    ui,
                    Icon::Command,
                    &format!("Command palette · {}", self.shortcut(Action::Commands)),
                    self.palette.is_some(),
                    p,
                )
                .clicked()
                {
                    self.action(Action::Commands);
                }
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    if icon_button(
                        ui,
                        Icon::Settings,
                        &format!(
                            "Appearance & settings · {}",
                            self.shortcut(Action::Settings)
                        ),
                        false,
                        p,
                    )
                    .clicked()
                    {
                        self.action(Action::Settings);
                    }
                    if icon_button(
                        ui,
                        Icon::Play,
                        &format!("Output panel · {}", self.shortcut(Action::Output)),
                        self.settings.show_output,
                        p,
                    )
                    .clicked()
                    {
                        self.action(Action::Output);
                    }
                });
            });
    }
    pub(super) fn tabs(&mut self, ui: &mut egui::Ui, p: Palette) {
        let mut close = None;
        egui::Frame::new()
            .fill(p.surface)
            .inner_margin(egui::Margin::symmetric(6, 0))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("open-tabs")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(w) = &mut self.workspace {
                                for (i, tab) in w.tabs.iter().enumerate() {
                                    let selected = w.active == i;
                                    let label = format!(
                                        "{}{}",
                                        tab.name(),
                                        if tab.cached_dirty { "  •" } else { "" }
                                    );
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label).color(if selected {
                                                    p.text
                                                } else {
                                                    p.muted
                                                }),
                                            )
                                            .min_size(Vec2::new(90., 30.))
                                            .fill(
                                                if selected {
                                                    p.canvas
                                                } else {
                                                    Color32::TRANSPARENT
                                                },
                                            ),
                                        )
                                        .on_hover_text(tab.document.path.display().to_string())
                                        .clicked()
                                    {
                                        w.active = i;
                                    }
                                    if icon_button(ui, Icon::Close, "Close tab", false, p).clicked()
                                    {
                                        close = Some(i);
                                    }
                                }
                            }
                        });
                    });
            });
        if let Some(i) = close
            && let Some(w) = &mut self.workspace
            && w.close_clean(i).is_err()
        {
            self.dialog = Some(Dialog::Close(i));
        }
    }
}
