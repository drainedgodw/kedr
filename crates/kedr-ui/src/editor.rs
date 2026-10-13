use crate::{Palette, Theme};
use egui::{
    Align2, Event, FontId, Galley, Id, ImeEvent, Key, Pos2, Rect, Sense, Stroke, TextFormat, Ui,
    Vec2,
    text::{CCursor, LayoutJob},
};
use kedr_core::Version;
use kedr_workbench::{EditorTab, Movement};
use std::{collections::BTreeMap, sync::Arc};
const PREVIEW_BYTES: usize = 8192;
struct Line {
    text: String,
    galley: Arc<Galley>,
    byte_start: usize,
    char_start: usize,
    limited: bool,
}
#[derive(Default)]
pub struct EditorCache {
    version: Option<Version>,
    font: f32,
    theme: Option<Theme>,
    lines: BTreeMap<usize, Line>,
    pub painted_rows: usize,
    pub last_rect: Option<Rect>,
    pub focus_next: bool,
    composition: String,
    composing: bool,
}
impl EditorCache {
    pub fn cached_rows(&self) -> usize {
        self.lines.len()
    }
    pub fn clear(&mut self) {
        self.version = None;
        self.lines.clear();
        self.composition.clear();
        self.composing = false;
    }
}
fn syntax(text: &str, font: f32, p: Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut at = 0;
    let keywords = [
        "fn", "pub", "use", "mod", "impl", "struct", "enum", "let", "mut", "self", "Self",
        "return", "if", "else", "match", "for", "while", "loop", "in", "const", "static", "move",
        "async", "await", "class", "def", "import", "from", "function", "export", "true", "false",
        "None", "Some",
    ];
    while at < text.len() {
        let rest = &text[at..];
        let first = rest.chars().next().unwrap();
        let (len, color) =
            if rest.starts_with("//") || (rest.starts_with('#') && !rest.starts_with("#[")) {
                (rest.len(), p.comment)
            } else if first == '"' || first == '\'' {
                let mut escaped = false;
                let mut n = first.len_utf8();
                for c in rest[first.len_utf8()..].chars() {
                    n += c.len_utf8();
                    if c == first && !escaped {
                        break;
                    }
                    escaped = c == '\\' && !escaped;
                }
                (n, p.green)
            } else if first.is_ascii_digit() {
                (
                    rest.chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_')
                        .map(char::len_utf8)
                        .sum(),
                    p.amber,
                )
            } else if first.is_alphabetic() || first == '_' {
                let n: usize = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .map(char::len_utf8)
                    .sum();
                (
                    n,
                    if keywords.contains(&&rest[..n]) {
                        p.purple
                    } else {
                        p.text
                    },
                )
            } else {
                (first.len_utf8(), p.text)
            };
        job.append(
            &text[at..at + len],
            0.,
            TextFormat {
                font_id: FontId::monospace(font),
                color,
                ..Default::default()
            },
        );
        at += len;
    }
    job
}
fn line_layout(tab: &EditorTab, row: usize, ui: &Ui, font: f32, p: Palette) -> Line {
    let range = tab.line_bounds(row);
    let mut begin = range.start;
    let mut char_start = 0;
    if tab.position().0 == row && tab.cursor > begin + PREVIEW_BYTES {
        begin = tab
            .cursor
            .saturating_sub(PREVIEW_BYTES / 2)
            .max(range.start);
        while !tab.document.buffer.is_char_boundary(begin) {
            begin -= 1;
        }
        char_start = tab.document.buffer.byte_to_position(begin).unwrap().1;
    }
    let mut end = range.end.min(begin + PREVIEW_BYTES);
    while !tab.document.buffer.is_char_boundary(end) {
        end -= 1;
    }
    let text = tab.document.buffer.slice(begin..end).unwrap_or_default();
    let galley = ui.fonts_mut(|f| f.layout_job(syntax(&text, font, p)));
    Line {
        text,
        galley,
        byte_start: begin,
        char_start,
        limited: begin != range.start || end != range.end,
    }
}
fn offset(line: &Line, index: usize) -> usize {
    line.byte_start
        + line
            .text
            .char_indices()
            .nth(index)
            .map_or(line.text.len(), |(b, _)| b)
}
fn input(tab: &mut EditorTab, cache: &mut EditorCache, ui: &Ui) -> Option<String> {
    let mut error = None;
    for event in ui.input(|i| i.events.clone()) {
        let action = match event {
            Event::Copy => {
                ui.ctx().copy_text(tab.selected_text());
                Ok(())
            }
            Event::Cut => {
                ui.ctx().copy_text(tab.selected_text());
                if tab.cursor != tab.anchor {
                    tab.insert("")
                } else {
                    Ok(())
                }
            }
            Event::Paste(text) => tab.insert(&text),
            Event::Text(text) if !cache.composing => tab.insert(&text),
            Event::Ime(ImeEvent::Enabled) => {
                cache.composing = true;
                Ok(())
            }
            Event::Ime(ImeEvent::Preedit(text)) => {
                cache.composition = text;
                Ok(())
            }
            Event::Ime(ImeEvent::Commit(text)) => {
                cache.composition.clear();
                cache.composing = false;
                tab.insert(&text)
            }
            Event::Ime(ImeEvent::Disabled) => {
                cache.composition.clear();
                cache.composing = false;
                Ok(())
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let movement = match key {
                    Key::ArrowLeft => Some(if modifiers.command {
                        Movement::WordLeft
                    } else {
                        Movement::Left
                    }),
                    Key::ArrowRight => Some(if modifiers.command {
                        Movement::WordRight
                    } else {
                        Movement::Right
                    }),
                    Key::ArrowUp => Some(Movement::Up),
                    Key::ArrowDown => Some(Movement::Down),
                    Key::Home => Some(if modifiers.command {
                        Movement::DocumentStart
                    } else {
                        Movement::Home
                    }),
                    Key::End => Some(if modifiers.command {
                        Movement::DocumentEnd
                    } else {
                        Movement::End
                    }),
                    _ => None,
                };
                if let Some(m) = movement {
                    tab.move_cursor(m, modifiers.shift);
                    Ok(())
                } else {
                    match key {
                        Key::A if modifiers.command => {
                            tab.select_all();
                            Ok(())
                        }
                        Key::Z if modifiers.command => tab.undo(modifiers.shift),
                        Key::Y if modifiers.command => tab.undo(true),
                        Key::Backspace if !modifiers.command && !modifiers.alt => tab.backspace(),
                        Key::Delete if !modifiers.command && !modifiers.alt => tab.delete(),
                        Key::Enter if !cache.composing && !modifiers.command && !modifiers.alt => {
                            tab.enter()
                        }
                        Key::Tab if !cache.composing && !modifiers.command && !modifiers.alt => {
                            if tab.cursor != tab.anchor || modifiers.shift {
                                tab.indent(modifiers.shift)
                            } else {
                                tab.insert("    ")
                            }
                        }
                        _ => Ok(()),
                    }
                }
            }
            _ => Ok(()),
        };
        if let Err(e) = action {
            error = Some(e);
        }
    }
    error
}
pub fn editor(
    ui: &mut Ui,
    tab: &mut EditorTab,
    cache: &mut EditorCache,
    font: f32,
    theme: Theme,
    enabled: bool,
) -> Option<String> {
    let p = theme.palette();
    let version = tab.document.buffer.version();
    if cache.version != Some(version) || cache.font != font || cache.theme != Some(theme) {
        cache.lines.clear();
        cache.version = Some(version);
        cache.font = font;
        cache.theme = Some(theme);
    }
    if let Some(line) = cache.lines.get(&tab.position().0)
        && line.limited
        && (tab.cursor < line.byte_start || tab.cursor > line.byte_start + line.text.len())
    {
        cache.lines.remove(&tab.position().0);
    }
    let height = (font * 1.65).ceil();
    let gutter = 72.;
    let padding = 20.;
    let total = tab.document.buffer.line_count();
    let id = Id::new(("kedr-code", &tab.document.path));
    let mut scroll = egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false]);
    let follow = tab.follow_cursor;
    tab.follow_cursor = false;
    if follow {
        let rows = (ui.available_height() / height) as usize;
        scroll = scroll
            .vertical_scroll_offset(tab.position().0.saturating_sub(rows / 3) as f32 * height);
    }
    cache.painted_rows = 0;
    let mut error = None;
    let spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    scroll.show_rows(ui, height, total, |ui, rows| {
        cache.painted_rows = rows.len();
        // Keep nearby rows instead of flushing every cached galley at 256 rows.
        // Scrolling away prunes distant entries; returning nearby reuses layout.
        let keep = rows.start.saturating_sub(96)..rows.end.saturating_add(96);
        cache.lines.retain(|row, _| keep.contains(row));
        for row in rows.clone() {
            cache
                .lines
                .entry(row)
                .or_insert_with(|| line_layout(tab, row, ui, font, p));
        }
        let width = cache
            .lines
            .values()
            .map(|l| l.galley.size().x + gutter + padding * 2.)
            .fold(ui.available_width(), f32::max);
        let (_, rect) = ui.allocate_space(Vec2::new(width, rows.len() as f32 * height));
        let response = ui.interact(
            rect,
            id,
            if enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        cache.last_rect = Some(rect);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, "Code editor")
        });
        if enabled && (cache.focus_next || response.clicked()) {
            response.request_focus();
            cache.focus_next = false;
        }
        if enabled
            && (response.clicked() || response.dragged())
            && let Some(pos) = response.interact_pointer_pos()
        {
            let row = (rows.start + ((pos.y - rect.top()) / height).max(0.) as usize)
                .min(rows.end.saturating_sub(1));
            if let Some(line) = cache.lines.get(&row) {
                let y = rect.top() + (row - rows.start) as f32 * height;
                let cc = line
                    .galley
                    .cursor_from_pos(pos - Pos2::new(rect.left() + gutter + padding, y));
                let at = offset(line, cc.index);
                let extend = ui.input(|i| i.modifiers.shift) || response.dragged();
                tab.set_cursor(at, extend);
                tab.follow_cursor = false;
                ui.ctx().request_repaint();
            }
        }
        let cursor_row = tab.position().0;
        let selection = tab.selection();
        let painter = ui.painter_at(rect.intersect(ui.clip_rect()));
        for row in rows.clone() {
            let line = &cache.lines[&row];
            let top = rect.top() + (row - rows.start) as f32 * height;
            let start = Pos2::new(rect.left() + gutter + padding, top + 3.);
            if row == cursor_row {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left(), top),
                        Pos2::new(rect.right(), top + height),
                    ),
                    0.,
                    p.raised.gamma_multiply(0.45),
                );
            }
            painter.text(
                Pos2::new(rect.left() + gutter - 16., top + height / 2.),
                Align2::RIGHT_CENTER,
                (row + 1).to_string(),
                FontId::monospace(14.),
                if row == cursor_row {
                    p.accent
                } else {
                    p.comment
                },
            );
            painter.line_segment(
                [
                    Pos2::new(rect.left() + gutter, top),
                    Pos2::new(rect.left() + gutter, top + height),
                ],
                Stroke::new(1.0_f32, p.border),
            );
            let byte_end = line.byte_start + line.text.len();
            if selection.start < selection.end
                && selection.end > line.byte_start
                && selection.start <= byte_end
            {
                let a = selection.start.max(line.byte_start);
                let b = selection.end.min(byte_end);
                let ai = line.text[..a - line.byte_start].chars().count();
                let bi = line.text[..b - line.byte_start].chars().count();
                let x1 = line.galley.pos_from_cursor(CCursor::new(ai)).left();
                let mut x2 = line.galley.pos_from_cursor(CCursor::new(bi)).left();
                if selection.end > byte_end {
                    x2 += font * 0.65;
                }
                painter.rect_filled(
                    Rect::from_min_max(
                        start + Vec2::new(x1, 0.),
                        start + Vec2::new(x2, height - 3.),
                    ),
                    2.,
                    p.accent_soft,
                );
            }
            painter.galley(start, line.galley.clone(), p.text);
            if line.limited {
                painter.text(
                    start + Vec2::new(line.galley.size().x + 12., 0.),
                    Align2::LEFT_TOP,
                    "… bounded preview",
                    FontId::proportional(14.),
                    p.muted,
                );
            }
            if row == cursor_row && tab.cursor >= line.byte_start && tab.cursor <= byte_end {
                let col = tab.position().1.saturating_sub(line.char_start);
                let x = line.galley.pos_from_cursor(CCursor::new(col)).left();
                let caret =
                    Rect::from_min_size(start + Vec2::new(x, 0.), Vec2::new(1.5, height - 4.));
                if enabled && response.has_focus() {
                    painter.rect_filled(caret, 0., p.accent);
                    ui.ctx().output_mut(|o| {
                        o.ime = Some(egui::output::IMEOutput {
                            rect: rect.intersect(ui.clip_rect()),
                            cursor_rect: caret,
                        })
                    });
                }
                if !cache.composition.is_empty() {
                    painter.text(
                        caret.left_top(),
                        Align2::LEFT_TOP,
                        &cache.composition,
                        FontId::monospace(font),
                        p.accent,
                    );
                }
                if follow {
                    ui.scroll_to_rect(caret, Some(egui::Align::Center));
                }
            }
        }
        if enabled && response.has_focus() {
            let before = (tab.cursor, tab.anchor);
            error = input(tab, cache, ui);
            if tab.document.buffer.version() != version || before != (tab.cursor, tab.anchor) {
                ui.ctx().request_repaint();
            }
        }
    });
    ui.spacing_mut().item_spacing.y = spacing;
    error
}
