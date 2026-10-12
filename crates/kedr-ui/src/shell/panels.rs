use super::*;
impl Shell {
    pub(super) fn sidebar(&mut self, ctx: &Context, p: Palette) {
        let pane = self.pane;
        let width = self
            .settings
            .sidebar_width
            .min((ctx.content_rect().width() * 0.34).max(220.));
        egui::SidePanel::left("kedr-sidebar")
            .default_width(width)
            .width_range(220.0..=420.0)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(16))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(if pane == Pane::Files {
                        "PROJECT"
                    } else {
                        "SEARCH"
                    })
                    .color(p.muted)
                    .strong(),
                );
                ui.add_space(12.);
                if pane == Pane::Files {
                    let r = ui.add(
                        singleline(&mut self.file_filter)
                            .hint_text("Filter files…")
                            .char_limit(256)
                            .desired_width(f32::INFINITY),
                    );
                    if r.changed() {
                        self.rebuild_nav();
                    }
                    if let Some(w) = &self.workspace {
                        ui.label(
                            RichText::new(format!(
                                "{} · {} files",
                                w.name(),
                                w.project.files.len()
                            ))
                            .color(p.muted),
                        );
                        if w.project.truncated {
                            ui.label(RichText::new("Scan limit reached").color(p.amber));
                        }
                    }
                    ui.add_space(8.);
                    let mut choose = None;
                    let mut folder = None;
                    egui::ScrollArea::vertical().id_salt("file-tree").show_rows(
                        ui,
                        44.,
                        self.nav.len(),
                        |ui, rows| {
                            for i in rows {
                                let e = &self.nav[i];
                                ui.horizontal(|ui| {
                                    ui.add_space((e.depth.min(5) * 12) as f32);
                                    let selected = self.active_path.as_ref() == Some(&e.path);
                                    let label = e.label.clone();
                                    let response = ui.add(
                                        egui::Button::new("")
                                            .selected(selected)
                                            .frame(selected)
                                            .min_size(Vec2::new(ui.available_width(), 44.)),
                                    );
                                    if e.folder {
                                        let center =
                                            response.rect.left_center() + Vec2::new(13.0, 0.0);
                                        let points = if self.expanded.contains(&e.path) {
                                            [
                                                center + Vec2::new(-3.0, -2.0),
                                                center + Vec2::new(0.0, 2.0),
                                                center + Vec2::new(3.0, -2.0),
                                            ]
                                        } else {
                                            [
                                                center + Vec2::new(-2.0, -3.0),
                                                center + Vec2::new(2.0, 0.0),
                                                center + Vec2::new(-2.0, 3.0),
                                            ]
                                        };
                                        ui.painter().line_segment(
                                            [points[0], points[1]],
                                            egui::Stroke::new(1.5_f32, p.muted),
                                        );
                                        ui.painter().line_segment(
                                            [points[1], points[2]],
                                            egui::Stroke::new(1.5_f32, p.muted),
                                        );
                                    }
                                    let text = ui.fonts_mut(|f| {
                                        f.layout_no_wrap(
                                            label.clone(),
                                            FontId::proportional(14.0),
                                            if selected { p.accent } else { p.text },
                                        )
                                    });
                                    ui.painter_at(response.rect).galley(
                                        response.rect.left_center()
                                            + Vec2::new(
                                                if e.folder { 28.0 } else { 12.0 },
                                                -text.size().y / 2.0,
                                            ),
                                        text,
                                        p.text,
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            &label,
                                        )
                                    });
                                    if response
                                        .on_hover_text(e.path.display().to_string())
                                        .clicked()
                                    {
                                        if e.folder {
                                            folder = Some(e.path.clone());
                                        } else {
                                            choose = Some(e.path.clone());
                                        }
                                    }
                                });
                            }
                        },
                    );
                    if let Some(path) = folder {
                        if !self.expanded.remove(&path) {
                            self.expanded.insert(path);
                        }
                        self.rebuild_nav();
                    }
                    if let Some(path) = choose {
                        self.pending_hit = None;
                        self.effects.push(Effect::OpenFile(path));
                    }
                } else {
                    let r = ui.add(
                        singleline(&mut self.search)
                            .hint_text("Search saved files…")
                            .char_limit(256)
                            .desired_width(f32::INFINITY),
                    );
                    if self.search_focus {
                        r.request_focus();
                        self.search_focus = false;
                    }
                    if (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                        || ui.button("Search project").clicked()
                    {
                        self.search_loading = true;
                        self.effects.push(Effect::Search(self.search.clone()));
                    }
                    ui.label(
                        RichText::new("Literal · case-sensitive · saved files").color(p.muted),
                    );
                    if self.search_loading {
                        ui.spinner();
                    }
                    let mut selected = None;
                    if let Some(r) = &self.search_report {
                        ui.label(format!(
                            "{} matches · {} files",
                            r.hits.len(),
                            r.files_scanned
                        ));
                        if r.limited {
                            ui.label(
                                RichText::new("Results limited; narrow your query").color(p.amber),
                            );
                        }
                        if r.skipped > 0 {
                            ui.label(format!("{} files skipped", r.skipped));
                        }
                        egui::ScrollArea::vertical().show_rows(
                            ui,
                            66.,
                            r.hits.len(),
                            |ui, rows| {
                                for i in rows {
                                    let h = &r.hits[i];
                                    let name =
                                        h.path.file_name().unwrap_or_default().to_string_lossy();
                                    let title = format!("{}:{}", name, h.line + 1);
                                    let response = ui.add(
                                        egui::Button::new("")
                                            .min_size(Vec2::new(ui.available_width(), 60.)),
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            &title,
                                        )
                                    });
                                    let width = (response.rect.width() - 24.).max(1.);
                                    let painter =
                                        ui.painter().with_clip_rect(response.rect.shrink(8.));
                                    for (text, y, color) in
                                        [(&title, 10., p.text), (&h.preview, 32., p.muted)]
                                    {
                                        let mut job = egui::text::LayoutJob::simple_singleline(
                                            text.clone(),
                                            FontId::proportional(14.),
                                            color,
                                        );
                                        job.wrap.max_width = width;
                                        job.wrap.max_rows = 1;
                                        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                                        painter.galley(
                                            response.rect.min + Vec2::new(12., y),
                                            galley,
                                            color,
                                        );
                                    }
                                    let clicked = response.clicked();
                                    response.on_hover_text(format!(
                                        "{}\n{}",
                                        h.path.display(),
                                        h.preview
                                    ));
                                    if clicked {
                                        selected = Some(h.clone());
                                    }
                                }
                            },
                        );
                    }
                    if let Some(hit) = selected {
                        self.effects.push(Effect::OpenFile(hit.path.clone()));
                        self.pending_hit = Some(hit);
                    }
                }
            });
    }
    pub(super) fn find_bar(&mut self, ui: &mut egui::Ui, p: Palette) {
        let (mut next, mut previous, mut all) = (false, false, false);
        let compact = ui.available_width() < 600.;
        egui::Frame::new()
            .fill(p.surface)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let r = ui.add(
                        singleline(&mut self.find)
                            .hint_text("Find in current file")
                            .char_limit(4096)
                            .desired_width(if compact { 120. } else { 190. }),
                    );
                    if self.find_focus {
                        r.request_focus();
                        self.find_focus = false;
                    }
                    next |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    previous |= ui
                        .button("Prev")
                        .on_hover_text(self.shortcut(Action::FindPrevious))
                        .clicked();
                    next |= ui
                        .button("Next")
                        .on_hover_text(self.shortcut(Action::FindNext))
                        .clicked();
                    ui.add(
                        singleline(&mut self.replacement)
                            .hint_text("Replace with")
                            .char_limit(4096)
                            .desired_width(if compact { 90. } else { 150. }),
                    );
                    all |= ui.button("Replace all").clicked();
                    if icon_button(ui, Icon::Close, "Close find bar · Escape", false, p).clicked()
                    {
                        self.find_open = false;
                        self.editor_cache.focus_next = true;
                    }
                });
            });
        if let Some(tab) = self.workspace.as_mut().and_then(Workspace::active_mut) {
            if next {
                self.status = if tab.find_next(&self.find) {
                    "Match selected"
                } else {
                    "No match"
                }
                .into();
            }
            if previous {
                self.status = if tab.find_previous(&self.find) {
                    "Match selected"
                } else {
                    "No match"
                }
                .into();
            }
            if all {
                match tab.replace_all(&self.find, &self.replacement) {
                    Ok(n) => self.status = format!("Replaced {n} matches"),
                    Err(e) => self.error(e),
                }
            }
        }
    }
    pub(super) fn output(&mut self, ctx: &Context, p: Palette) {
        egui::TopBottomPanel::bottom("kedr-output")
            .default_height(220.)
            .height_range(140.0..=420.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(16))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("OUTPUT").strong().color(p.muted));
                    if let Some(t) = &self.task {
                        ui.label(t.kind.label());
                        let phase = t.phase();
                        ui.label(match &phase {
                            TaskPhase::Running => "Running",
                            TaskPhase::Success => "Succeeded",
                            TaskPhase::Failed(_) => "Failed",
                            TaskPhase::Stopped => "Stopped",
                            TaskPhase::Error(_) => "Could not start",
                        })
                        .on_hover_text(format!("{phase:?}"));
                        if phase == TaskPhase::Running && ui.button("Stop").clicked() {
                            t.stop();
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Hide").clicked() {
                            self.settings.show_output = false;
                        }
                        if ui.button("Test").clicked() {
                            self.action(Action::Test);
                        }
                    });
                });
                if let Some(t) = &self.task {
                    let mut output = t.output();
                    egui::ScrollArea::both()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut output)
                                    .font(FontId::monospace(14.))
                                    .interactive(false)
                                    .desired_width(f32::INFINITY),
                            );
                        });
                } else {
                    ui.label("Run Cargo check or Cargo test to see real process output here.");
                    ui.label(
                        RichText::new("No project code runs until you explicitly allow it.")
                            .color(p.muted),
                    );
                }
            });
    }
    pub(super) fn welcome(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.add_space(ui.available_height() * 0.14);
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 600.).max(24.) / 2.);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.product.to_uppercase())
                        .color(p.accent)
                        .strong(),
                );
                ui.add_space(16.);
                ui.label(
                    RichText::new("A clear space\nfor serious work.")
                        .size(42.)
                        .strong(),
                );
                ui.add_space(16.);
                ui.label(
                    RichText::new("Your code. Your machine. No subscriptions.")
                        .size(18.)
                        .color(p.muted),
                );
                ui.add_space(32.);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Open workspace").strong())
                                .fill(p.accent_soft)
                                .min_size(Vec2::new(180., 48.)),
                        )
                        .clicked()
                    {
                        self.action(Action::OpenProject);
                    }
                    if ui.button("Commands").clicked() {
                        self.action(Action::Commands);
                    }
                });
                ui.add_space(32.);
                if !self.recent.is_empty() {
                    ui.label(RichText::new("RECENT WORKSPACES").color(p.muted));
                    let mut chosen = None;
                    for path in self.recent.iter().take(4) {
                        if ui
                            .add(
                                egui::Button::new(path.display().to_string())
                                    .wrap_mode(egui::TextWrapMode::Truncate)
                                    .min_size(Vec2::new(580., 44.)),
                            )
                            .clicked()
                        {
                            chosen = Some(path.clone());
                        }
                    }
                    if let Some(path) = chosen {
                        self.request_project(path);
                    }
                }
                ui.add_space(24.);
                ui.label(
                    RichText::new("Early desktop build · semantic tools are still ahead")
                        .color(p.muted),
                );
            });
        });
    }
}
