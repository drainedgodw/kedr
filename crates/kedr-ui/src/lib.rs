//! One shared visual language and workbench shell for the Kedr product family.
mod editor;
mod keymap;
pub use keymap::{Binding, Keymap, bindings, hint};
mod shell;
mod theme;
pub use editor::{EditorCache, editor};
use egui::{Color32, Sense, Stroke, Ui, Vec2};
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
}
/// Shared single-line input with the same pointer target as other controls.
pub(crate) fn singleline(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text)
        .min_size(Vec2::new(0.0, 44.0))
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
            .min_size(Vec2::splat(44.))
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
    let (r, _) = ui.allocate_exact_size(Vec2::new(32., 32.), Sense::hover());
    ui.painter().rect_filled(r, 8., p.accent);
    let point = |x: f32, y: f32| r.min + Vec2::new(x, y);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![point(16., 4.), point(8., 13.), point(24., 13.)],
        p.canvas,
        Stroke::NONE,
    ));
    ui.painter().add(egui::Shape::convex_polygon(
        vec![point(16., 10.), point(5., 22.), point(27., 22.)],
        p.canvas,
        Stroke::NONE,
    ));
    ui.painter().rect_filled(
        egui::Rect::from_min_max(point(14., 20.), point(18., 28.)),
        0.,
        p.canvas,
    );
    ui.label(egui::RichText::new("KEDR").size(18.).strong().color(p.text));
}
