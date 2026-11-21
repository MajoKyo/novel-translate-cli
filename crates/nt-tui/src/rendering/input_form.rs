use super::{layout_areas, terminal_size_supported};
use crate::navigation::Screen;
use crate::state::{AppState, FocusRegion};
use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputControl {
    Text,
    Primary,
    Back,
    Discard,
}

impl InputControl {
    pub(crate) fn next(self, has_text: bool, reverse: bool) -> Self {
        let controls: &[Self] = if has_text {
            &[Self::Text, Self::Primary, Self::Back, Self::Discard]
        } else {
            &[Self::Primary, Self::Back, Self::Discard]
        };
        let index = controls
            .iter()
            .position(|control| *control == self)
            .unwrap_or(0);
        let next = if reverse {
            (index + controls.len() - 1) % controls.len()
        } else {
            (index + 1) % controls.len()
        };
        controls[next]
    }
}

pub(crate) struct InputForm {
    pub(crate) title: &'static str,
    pub(crate) label: &'static str,
    pub(crate) text: Option<String>,
    pub(crate) message: String,
    pub(crate) primary: &'static str,
    pub(crate) focused: InputControl,
}

struct FormLayout {
    area: Rect,
    inner: Rect,
    field: Option<Rect>,
    buttons: Vec<(InputControl, Rect, String)>,
}

fn layout(area: Rect, state: &AppState, form: &InputForm) -> Option<FormLayout> {
    if !terminal_size_supported(area)
        || state.help_open()
        || state.current_screen() != Screen::VolumesAndChapters
        || state.focus_region() != FocusRegion::Workspace
    {
        return None;
    }
    let panel = layout_areas(area).2;
    let inner = Block::default().borders(Borders::ALL).inner(panel);
    let field = form
        .text
        .as_ref()
        .map(|_| Rect::new(inner.x, inner.y + 1, inner.width, 3));
    let row = if field.is_some() {
        inner.y + 5
    } else {
        inner.bottom().saturating_sub(2)
    };
    let mut x = inner.x;
    let mut buttons = Vec::new();
    for (control, label) in [
        (InputControl::Primary, form.primary),
        (InputControl::Back, "Trở về"),
        (InputControl::Discard, "Bỏ nháp"),
    ] {
        let marker = if control == form.focused {
            "› "
        } else {
            "  "
        };
        let text = format!("{marker}[{label}]");
        let width = text.cell_width().min(inner.right().saturating_sub(x));
        buttons.push((control, Rect::new(x, row, width, 1), text));
        x = x.saturating_add(width).saturating_add(1);
    }
    Some(FormLayout {
        area: panel,
        inner,
        field,
        buttons,
    })
}

fn suffix(text: &str, capacity: u16) -> (&str, u16) {
    let mut start = text.len();
    let mut used = 0;
    for (offset, grapheme) in text.grapheme_indices(true).rev() {
        let width = grapheme.cell_width();
        if width > capacity.saturating_sub(used) {
            break;
        }
        used += width;
        start = offset;
    }
    (&text[start..], used)
}

fn window(text: &str, width: u16) -> (String, u16) {
    let capacity = width.saturating_sub(1);
    let (visible, used) = suffix(text, capacity);
    if visible.len() == text.len() || capacity == 0 {
        return (visible.to_owned(), used);
    }
    let (visible, used) = suffix(text, capacity - 1);
    (format!("‹{visible}"), used + 1)
}

pub(crate) fn render(
    buffer: &mut Buffer,
    area: Rect,
    state: &AppState,
    form: &InputForm,
) -> Option<(u16, u16)> {
    let layout = layout(area, state, form)?;
    Clear.render(layout.area, buffer);
    Block::default()
        .title(form.title)
        .borders(Borders::ALL)
        .render(layout.area, buffer);
    let mut caret = None;
    if let (Some(field), Some(text)) = (layout.field, form.text.as_ref()) {
        Paragraph::new("Nhập nội dung rồi chọn Xem trước.").render(
            Rect::new(layout.inner.x, layout.inner.y, layout.inner.width, 1),
            buffer,
        );
        let selected = form.focused == InputControl::Text;
        let label = format!("{}{}", if selected { "› " } else { "" }, form.label);
        let block = Block::default()
            .title(label)
            .borders(Borders::ALL)
            .border_style(if selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });
        let inside = block.inner(field);
        block.render(field, buffer);
        let (visible, column) = window(text, inside.width);
        Paragraph::new(visible).render(inside, buffer);
        if selected && inside.width > 0 && inside.height > 0 {
            caret = Some((inside.x + column, inside.y));
        }
        let message = Rect::new(
            layout.inner.x,
            layout.inner.y + 6,
            layout.inner.width,
            layout.inner.height.saturating_sub(6),
        );
        Paragraph::new(form.message.as_str())
            .wrap(Wrap { trim: true })
            .render(message, buffer);
    } else {
        let content = Rect::new(
            layout.inner.x,
            layout.inner.y,
            layout.inner.width,
            layout.inner.height.saturating_sub(3),
        );
        Paragraph::new(form.message.as_str())
            .wrap(Wrap { trim: true })
            .scroll((state.scroll_offset(), 0))
            .render(content, buffer);
    }
    for (control, rect, text) in layout.buttons {
        let style = if control == form.focused {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
        };
        Paragraph::new(text).style(style).render(rect, buffer);
    }
    let footer = layout_areas(area).3;
    Clear.render(footer, buffer);
    Paragraph::new("Tab chọn ô/nút · Enter kích hoạt · Esc trở về · F1 trợ giúp")
        .render(footer, buffer);
    caret
}

pub(crate) fn hit(
    area: Rect,
    state: &AppState,
    form: &InputForm,
    x: u16,
    y: u16,
) -> Option<InputControl> {
    let layout = layout(area, state, form)?;
    let contains = |rect: Rect| x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom();
    if layout.field.is_some_and(contains) {
        return Some(InputControl::Text);
    }
    layout
        .buttons
        .into_iter()
        .find_map(|(control, rect, _)| contains(rect).then_some(control))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn form() -> InputForm {
        InputForm {
            title: "Đổi tên chương",
            label: "Tên chương mới:",
            text: Some("Tiếng Việt 日本 a\u{301}".to_owned()),
            message: String::new(),
            primary: "Xem trước",
            focused: InputControl::Text,
        }
    }
    fn state() -> AppState {
        let mut state = AppState::default();
        state.navigate_to(Screen::VolumesAndChapters);
        state.set_workspace_controls(true);
        state.focus_workspace();
        state
    }
    #[test]
    fn field_window_preserves_graphemes_and_reserves_the_caret_cell() {
        assert_eq!(window("AB日本abc", 8), ("‹本abc".to_owned(), 6));
        assert_eq!(window("abca\u{301}", 4), ("‹ca\u{301}".to_owned(), 3));
        assert_eq!(window("日本", 1), (String::new(), 0));
        assert_eq!(window("  a", 8), ("  a".to_owned(), 3));
    }
    #[test]
    fn render_and_mouse_share_bounds_and_keep_the_header_unchanged() {
        for (width, height) in [(80, 24), (120, 32)] {
            let area = Rect::new(0, 0, width, height);
            let state = state();
            let form = form();
            let mut buffer = Buffer::empty(area);
            crate::rendering::render(&mut buffer, area, &state);
            let header = buffer.content[..usize::from(width) * 3].to_vec();
            let (x, y) = render(&mut buffer, area, &state, &form).unwrap();
            assert_eq!(&buffer.content[..usize::from(width) * 3], header.as_slice());
            assert_eq!(hit(area, &state, &form, x, y), Some(InputControl::Text));
            let geometry = layout(area, &state, &form).unwrap();
            let field = Block::default()
                .borders(Borders::ALL)
                .inner(geometry.field.unwrap());
            assert!(x >= field.x && x < field.right() && y >= field.y && y < field.bottom());
            for (control, rect, _) in geometry.buttons {
                assert_eq!(hit(area, &state, &form, rect.x, rect.y), Some(control));
            }
            assert_eq!(hit(area, &state, &form, 0, 0), None);
        }
    }
    #[test]
    fn button_focus_stays_visible_without_color_and_hides_the_caret() {
        let area = Rect::new(0, 0, 80, 24);
        let state = state();
        let mut form = form();
        form.focused = InputControl::Primary;
        let mut buffer = Buffer::empty(area);
        assert_eq!(render(&mut buffer, area, &state, &form), None);
        let geometry = layout(area, &state, &form).unwrap();
        let rect = geometry.buttons[0].1;
        assert_eq!(buffer[(rect.x, rect.y)].symbol(), "›");
        assert!(
            buffer[(rect.x, rect.y)]
                .modifier
                .contains(Modifier::BOLD | Modifier::REVERSED)
        );
    }
    #[test]
    fn help_and_unsupported_size_have_no_edit_targets_or_caret() {
        let form = form();
        let mut state = state();
        state.open_help();
        assert!(layout(Rect::new(0, 0, 80, 24), &state, &form).is_none());
        state.close_help();
        assert!(layout(Rect::new(0, 0, 72, 20), &state, &form).is_none());
        assert_eq!(InputControl::Text.next(true, true), InputControl::Discard);
        assert_eq!(InputControl::Discard.next(true, false), InputControl::Text);
        assert_eq!(
            InputControl::Discard.next(false, false),
            InputControl::Primary
        );
    }
}
