//! One shared visual language and workbench shell for the Kedr product family.
mod brand;
mod editor;
mod keymap;
pub use keymap::{Binding, Keymap, bindings, hint};
mod remap;
pub use remap::{
    Chord, ShortcutOverrides, override_id, resolved_bindings, resolved_hint, set_override,
    validate_overrides,
};
mod shell;
mod theme;
pub use editor::{EditorCache, editor};
use egui::{Color32, Stroke, Ui, Vec2};
pub use shell::{Action, Effect, Settings, Shell};
pub use theme::{Palette, Theme, apply_theme};
#[derive(Clone, Copy)]
pub enum Icon {
    Files,
    Search,
    Settings,
    Command,
    Play,
    Close,
    Back,
    Forward,
}
/// Shared single-line input with the same pointer target as other controls.
pub(crate) fn singleline(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text)
        .min_size(Vec2::new(0.0, 28.0))
        .vertical_align(egui::Align::Center)
}

pub fn icon_button(
    ui: &mut Ui,
    kind: Icon,
    label: &str,
    selected: bool,
    p: Palette,
) -> egui::Response {
    let r = ui.add(
        egui::Button::new("")
            .min_size(Vec2::splat(32.))
            .fill(if selected {
                p.accent_soft
            } else {
                Color32::TRANSPARENT
            }),
    );
    let c = r.rect.center();
    let color = if selected { p.accent } else { p.muted };
    let pt = |x: f32, y: f32| c + Vec2::new(x, y);
    let stroke = Stroke::new(1.6_f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        ui.painter()
            .line_segment([pt(a.0, a.1), pt(b.0, b.1)], stroke)
    };
    match kind {
        Icon::Back | Icon::Forward => {
            let sign = if matches!(kind, Icon::Back) { -1. } else { 1. };
            line((-6. * sign, 0.), (6. * sign, 0.));
            line((1. * sign, -5.), (6. * sign, 0.));
            line((1. * sign, 5.), (6. * sign, 0.));
        }
        Icon::Search => {
            ui.painter().circle_stroke(pt(-2., -2.), 5., stroke);
            line((2., 2.), (7., 7.));
        }
        Icon::Files => {
            ui.painter().rect_stroke(
                egui::Rect::from_min_max(pt(-6., -8.), pt(6., 8.)),
                2.,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((-3., -3.), (3., -3.));
            line((-3., 1.), (3., 1.));
            line((-3., 5.), (2., 5.));
        }
        Icon::Close => {
            line((-5., -5.), (5., 5.));
            line((-5., 5.), (5., -5.));
        }
        Icon::Play => {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![pt(-4., -7.), pt(7., 0.), pt(-4., 7.)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Settings => {
            ui.painter().circle_stroke(c, 5., stroke);
            ui.painter().circle_stroke(c, 2., stroke);
            for i in 0..8 {
                let v = Vec2::angled(i as f32 * std::f32::consts::TAU / 8.);
                ui.painter().line_segment([c + v * 6., c + v * 9.], stroke);
            }
        }
        Icon::Command => {
            for x in [-1., 1.] {
                for y in [-1., 1.] {
                    ui.painter().circle_stroke(pt(x * 5., y * 5.), 2.5, stroke);
                }
            }
            ui.painter().rect_stroke(
                egui::Rect::from_min_max(pt(-5., -5.), pt(5., 5.)),
                0.,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
    }
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    r.on_hover_text(label)
}
pub fn wordmark(ui: &mut Ui, p: Palette) {
    let id = egui::Id::new("kedr-brand-texture");
    let cached = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(id));
    let texture = cached.unwrap_or_else(|| {
        let texture = ui.ctx().load_texture(
            "Kedr logo",
            egui::ColorImage::from_rgba_unmultiplied([32, 32], brand::RGBA),
            egui::TextureOptions::LINEAR,
        );
        ui.ctx().data_mut(|d| d.insert_temp(id, texture.clone()));
        texture
    });
    ui.add(egui::Image::new((texture.id(), Vec2::splat(32.))));
    ui.label(egui::RichText::new("KEDR").size(18.).strong().color(p.text));
}

/// Compact workbench row shared by palettes and recent workspaces.
pub(crate) struct ListRow<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub shortcut: &'a str,
    pub selected: bool,
}
pub(crate) fn list_row(ui: &mut Ui, row: ListRow<'_>, p: Palette) -> egui::Response {
    let height = if row.detail.is_empty() { 32. } else { 48. };
    let response = ui.add(
        egui::Button::new("")
            .selected(row.selected)
            .frame(row.selected)
            .min_size(Vec2::new(ui.available_width(), height)),
    );
    let rect = response.rect.shrink2(Vec2::new(10., 0.));
    let painter = ui.painter_at(rect);
    let trailing = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            row.shortcut.into(),
            egui::FontId::proportional(12.),
            p.muted,
        )
    });
    let reserve = if row.shortcut.is_empty() {
        0.
    } else {
        trailing.size().x + 20.
    };
    let width = (rect.width() - reserve).max(1.);
    for (text, y, color, size) in if row.detail.is_empty() {
        vec![(row.title, (height - 16.) / 2., p.text, 13.)]
    } else {
        vec![
            (row.title, 6., p.text, 13.),
            (row.detail, 26., p.muted, 12.),
        ]
    } {
        let mut job = egui::text::LayoutJob::simple_singleline(
            text.into(),
            egui::FontId::proportional(size),
            color,
        );
        job.wrap.max_width = width;
        job.wrap.max_rows = 1;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        painter.galley(rect.left_top() + Vec2::new(0., y), galley, color);
    }
    painter.galley(
        egui::pos2(
            rect.right() - trailing.size().x,
            rect.center().y - trailing.size().y / 2.,
        ),
        trailing,
        p.muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("{} {} {}", row.title, row.detail, row.shortcut),
        )
    });
    response
}
