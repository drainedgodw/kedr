use egui::{Color32, Context, FontId, Stroke, TextStyle, Vec2, Visuals};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub green: Color32,
    pub amber: Color32,
    pub purple: Color32,
    pub comment: Color32,
}
impl Theme {
    pub fn palette(self) -> Palette {
        let c = Color32::from_rgb;
        match self {
            Self::Dark => Palette {
                canvas: c(18, 22, 29),
                surface: c(25, 30, 39),
                raised: c(35, 42, 54),
                border: c(49, 58, 73),
                text: c(224, 230, 239),
                muted: c(168, 180, 198),
                accent: c(121, 181, 248),
                accent_soft: c(35, 53, 76),
                green: c(152, 208, 166),
                amber: c(233, 189, 139),
                purple: c(203, 173, 248),
                comment: c(145, 160, 180),
            },
            Self::Light => Palette {
                canvas: c(255, 255, 255),
                surface: c(245, 247, 250),
                raised: c(230, 236, 244),
                border: c(214, 221, 230),
                text: c(37, 47, 63),
                muted: c(93, 108, 126),
                accent: c(26, 99, 184),
                accent_soft: c(222, 237, 255),
                green: c(31, 112, 63),
                amber: c(146, 79, 9),
                purple: c(121, 60, 174),
                comment: c(95, 109, 126),
            },
        }
    }
}
pub fn apply_theme(ctx: &Context, theme: Theme) {
    let p = theme.palette();
    let mut v = if theme == Theme::Dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.surface;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.canvas;
    v.faint_bg_color = p.raised;
    v.override_text_color = Some(p.text);
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.window_corner_radius = 12.into();
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.noninteractive.bg_fill = p.surface;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.muted);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.hovered.bg_fill = p.accent_soft;
    v.widgets.hovered.weak_bg_fill = p.accent_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.active.bg_fill = p.accent_soft;
    v.widgets.active.weak_bg_fill = p.accent_soft;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.accent);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.corner_radius = 4.into();
    }
    let mut style = (*ctx.style()).clone();
    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(6., 4.);
    style.spacing.button_padding = Vec2::new(8., 4.);
    style.spacing.interact_size = Vec2::new(28., 28.);
    for text in [TextStyle::Body, TextStyle::Button, TextStyle::Small] {
        style.text_styles.insert(text, FontId::proportional(13.));
    }
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(15.));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(20.));
    ctx.set_style(style);
}
