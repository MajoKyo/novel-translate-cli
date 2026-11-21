use super::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::CellWidth;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Paragraph, Widget};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
struct Capture {
    bytes: Vec<u8>,
    writes: usize,
    flushes: usize,
}

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        self.writes += 1;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Pixel {
    text: Option<String>,
    fg: Color,
    bg: Color,
    modifier: Modifier,
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            text: Some(" ".into()),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
        }
    }
}

// Replay the terminal commands, independently of Buffer::diff. Expected cells
// come from a full render, so dropped styles, tails or wide cells are visible.
struct Screen {
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    style: Pixel,
    cells: Vec<Pixel>,
    cursor_visible: bool,
}

impl Screen {
    fn new(area: Rect) -> Self {
        assert_eq!((area.x, area.y), (0, 0));
        Self {
            width: usize::from(area.width),
            height: usize::from(area.height),
            x: 0,
            y: 0,
            style: Pixel::default(),
            cursor_visible: false,
            cells: vec![Pixel::default(); usize::from(area.width) * usize::from(area.height)],
        }
    }

    fn replay(&mut self, bytes: &[u8]) {
        let text = std::str::from_utf8(bytes).expect("UTF-8 terminal output");
        let mut remaining = text;
        while !remaining.is_empty() {
            if let Some(command) = remaining.strip_prefix("\x1b[") {
                let end = command
                    .find(|ch: char| ('@'..='~').contains(&ch))
                    .expect("CSI terminator");
                if &command[..end] == "?25" {
                    self.cursor_visible = match command.as_bytes()[end] {
                        b'h' => true,
                        b'l' => false,
                        other => panic!("Unexpected cursor command {other}"),
                    };
                    remaining = &command[end + 1..];
                    continue;
                }
                let arguments: Vec<u16> = command[..end]
                    .split(';')
                    .map(|value| {
                        if value.is_empty() {
                            0
                        } else {
                            value.parse().expect("CSI parameter")
                        }
                    })
                    .collect();
                match command.as_bytes()[end] {
                    b'H' => {
                        self.y = usize::from(arguments[0].saturating_sub(1));
                        self.x =
                            usize::from(arguments.get(1).copied().unwrap_or(1).saturating_sub(1));
                    }
                    b'J' => {
                        assert_eq!(arguments, [2]);
                        self.cells.fill(Pixel::default());
                    }
                    b'm' => self.sgr(&arguments),
                    other => panic!("Unexpected terminal command {other}"),
                }
                remaining = &command[end + 1..];
            } else {
                let end = remaining.find('\x1b').unwrap_or(remaining.len());
                assert!(end > 0, "Unknown escape sequence");
                for glyph in remaining[..end].graphemes(true) {
                    self.print(glyph);
                }
                remaining = &remaining[end..];
            }
        }
    }

    fn sgr(&mut self, args: &[u16]) {
        let mut index = 0;
        while index < args.len() {
            let code = args[index];
            match code {
                0 => self.style = Pixel::default(),
                1 => self.style.modifier.insert(Modifier::BOLD),
                2 => self.style.modifier.insert(Modifier::DIM),
                3 => self.style.modifier.insert(Modifier::ITALIC),
                4 => self.style.modifier.insert(Modifier::UNDERLINED),
                5 => self.style.modifier.insert(Modifier::SLOW_BLINK),
                6 => self.style.modifier.insert(Modifier::RAPID_BLINK),
                7 => self.style.modifier.insert(Modifier::REVERSED),
                8 => self.style.modifier.insert(Modifier::HIDDEN),
                9 => self.style.modifier.insert(Modifier::CROSSED_OUT),
                22 => self.style.modifier.remove(Modifier::BOLD | Modifier::DIM),
                23 => self.style.modifier.remove(Modifier::ITALIC),
                24 => self.style.modifier.remove(Modifier::UNDERLINED),
                25 => self
                    .style
                    .modifier
                    .remove(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK),
                27 => self.style.modifier.remove(Modifier::REVERSED),
                28 => self.style.modifier.remove(Modifier::HIDDEN),
                29 => self.style.modifier.remove(Modifier::CROSSED_OUT),
                30..=37 => self.style.fg = Color::Indexed(u8::try_from(code - 30).unwrap()),
                40..=47 => self.style.bg = Color::Indexed(u8::try_from(code - 40).unwrap()),
                90..=97 => self.style.fg = Color::Indexed(u8::try_from(code - 90 + 8).unwrap()),
                100..=107 => self.style.bg = Color::Indexed(u8::try_from(code - 100 + 8).unwrap()),
                39 => self.style.fg = Color::Reset,
                49 => self.style.bg = Color::Reset,
                59 => {} // Reset underline color; these fixtures use its default.
                38 | 48 => {
                    let color = match args[index + 1] {
                        5 => {
                            let color = Color::Indexed(u8::try_from(args[index + 2]).unwrap());
                            index += 2;
                            color
                        }
                        2 => {
                            let color = Color::Rgb(
                                u8::try_from(args[index + 2]).unwrap(),
                                u8::try_from(args[index + 3]).unwrap(),
                                u8::try_from(args[index + 4]).unwrap(),
                            );
                            index += 4;
                            color
                        }
                        other => panic!("Unknown color encoding {other}"),
                    };
                    if code == 38 {
                        self.style.fg = color;
                    } else {
                        self.style.bg = color;
                    }
                }
                other => panic!("Unknown SGR parameter {other}"),
            }
            index += 1;
        }
    }

    fn erase_glyph_at(&mut self, x: usize) {
        let mut leading = x;
        while leading > 0 && self.cells[self.y * self.width + leading].text.is_none() {
            leading -= 1;
        }
        let width = self.cells[self.y * self.width + leading]
            .text
            .as_ref()
            .expect("leading cell")
            .cell_width()
            .max(1);
        for column in leading..(leading + usize::from(width)).min(self.width) {
            self.cells[self.y * self.width + column] = Pixel::default();
        }
    }

    fn print(&mut self, glyph: &str) {
        if self.x == self.width {
            self.x = 0;
            self.y += 1;
        }
        let width = usize::from(glyph.cell_width().max(1));
        assert!(self.y < self.height && self.x + width <= self.width);
        for column in self.x..self.x + width {
            self.erase_glyph_at(column);
        }
        let index = self.y * self.width + self.x;
        self.cells[index] = Pixel {
            text: Some(glyph.into()),
            ..self.style.clone()
        };
        for column in 1..width {
            self.cells[index + column] = Pixel {
                text: None,
                ..self.style.clone()
            };
        }
        self.x += width;
    }

    fn assert_matches(&self, full: &Buffer) {
        for y in 0..full.area.height {
            let mut x = 0;
            while x < full.area.width {
                let expected = &full[(x, y)];
                let actual = &self.cells[usize::from(y) * self.width + usize::from(x)];
                assert_eq!(
                    actual.text.as_deref(),
                    Some(expected.symbol()),
                    "symbol at ({x},{y})"
                );
                assert_eq!(actual.fg, canonical(expected.fg), "foreground at ({x},{y})");
                assert_eq!(actual.bg, canonical(expected.bg), "background at ({x},{y})");
                assert_eq!(actual.modifier, expected.modifier, "style at ({x},{y})");
                let width = expected.symbol().cell_width().max(1);
                for continuation in 1..width {
                    assert!(
                        self.cells[usize::from(y) * self.width + usize::from(x + continuation)]
                            .text
                            .is_none()
                    );
                }
                x += width;
            }
        }
    }
}

fn canonical(color: Color) -> Color {
    if crossterm::style::Colored::ansi_color_disabled_memoized() {
        return Color::Reset;
    }
    let index = match color {
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        other => return other,
    };
    Color::Indexed(index)
}

fn buffer(width: u16, height: u16, text: &str) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut result = Buffer::empty(area);
    Paragraph::new(text).render(area, &mut result);
    result
}

fn draw(renderer: &mut FrameRenderer, screen: &mut Screen, buffer: &Buffer) -> Capture {
    let mut capture = Capture::default();
    renderer.draw(&mut capture, buffer).unwrap();
    if screen.width != usize::from(buffer.area.width)
        || screen.height != usize::from(buffer.area.height)
    {
        *screen = Screen::new(buffer.area);
    }
    screen.replay(&capture.bytes);
    screen.assert_matches(buffer);
    capture
}

#[test]
fn unchanged_frame_writes_nothing_and_selection_changes_use_one_batch() {
    let mut renderer = FrameRenderer::default();
    let mut state = crate::state::AppState::default();
    let area = Rect::new(0, 0, 80, 24);
    let first = super::super::render_shell_buffer(area, &state);
    let mut screen = Screen::new(area);
    let output = draw(&mut renderer, &mut screen, &first);
    assert!(output.bytes.windows(4).any(|value| value == b"\x1b[2J"));
    assert_eq!((output.writes, output.flushes), (1, 1));
    let same = draw(&mut renderer, &mut screen, &first);
    assert!(same.bytes.is_empty());
    assert_eq!((same.writes, same.flushes), (0, 0));
    crate::input::dispatch_event(
        &mut state,
        &Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        None,
        None,
    );
    let changed = super::super::render_shell_buffer(area, &state);
    assert_ne!(first, changed);
    let delta = draw(&mut renderer, &mut screen, &changed);
    assert!(!delta.bytes.windows(4).any(|value| value == b"\x1b[2J"));
    assert_eq!((delta.writes, delta.flushes), (1, 1));
    assert!(delta.bytes.len() < output.bytes.len() / 4);
}

#[test]
fn unicode_shorter_rows_and_style_only_changes_replay_like_full_frames() {
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(Rect::new(0, 0, 32, 4));
    for text in [
        "日本語  a\u{0301} Tiếng Việt\n日本",
        "日 a\u{0301}\n x",
        "a日本語\n短い行",
        "x\n ",
        "Tiếng Việt 👩‍💻\n🇻🇳 日本",
    ] {
        let full = buffer(32, 4, text);
        draw(&mut renderer, &mut screen, &full);
    }
    let mut styled = buffer(32, 4, "Giữ nguyên chữ Nhật 日本");
    draw(&mut renderer, &mut screen, &styled);
    styled[(3, 0)].set_style(
        Style::new()
            .fg(Color::Rgb(31, 42, 53))
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    );
    let delta = draw(&mut renderer, &mut screen, &styled);
    assert!(!delta.bytes.is_empty());
    assert!(!delta.bytes.windows(4).any(|value| value == b"\x1b[2J"));
}

#[test]
fn size_changes_and_explicit_surface_invalidation_repaint_completely() {
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(Rect::new(0, 0, 20, 4));
    for (width, height) in [(20, 4), (12, 2), (40, 6)] {
        let next = buffer(width, height, "日本\nTiếng Việt");
        let output = draw(&mut renderer, &mut screen, &next);
        assert!(output.bytes.windows(4).any(|value| value == b"\x1b[2J"));
        assert_eq!((output.writes, output.flushes), (1, 1));
        assert!(draw(&mut renderer, &mut screen, &next).bytes.is_empty());
    }
    renderer.invalidate();
    let output = draw(
        &mut renderer,
        &mut screen,
        &buffer(40, 6, "日本\nTiếng Việt"),
    );
    assert!(output.bytes.windows(4).any(|value| value == b"\x1b[2J"));
}

#[test]
fn partial_write_or_flush_failure_cannot_become_the_cached_frame() {
    struct Failing {
        remaining: usize,
        flush_error: bool,
    }
    impl Write for Failing {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("injected write failure"));
            }
            let count = self.remaining.min(bytes.len());
            self.remaining -= count;
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.flush_error {
                Err(io::Error::other("injected flush failure"))
            } else {
                Ok(())
            }
        }
    }
    for (remaining, flush_error) in [(0, false), (3, false), (usize::MAX, true)] {
        let mut renderer = FrameRenderer::default();
        renderer
            .draw(&mut Capture::default(), &buffer(20, 4, "first"))
            .unwrap();
        let next = buffer(20, 4, "次のフレーム");
        assert!(
            renderer
                .draw(
                    &mut Failing {
                        remaining,
                        flush_error
                    },
                    &next
                )
                .is_err()
        );
        let mut output = Capture::default();
        renderer.draw(&mut output, &next).unwrap();
        assert!(output.bytes.windows(4).any(|value| value == b"\x1b[2J"));
    }
}

#[test]
fn navigation_help_and_resize_replay_the_actual_shell_buffers() {
    let mut state = crate::state::AppState::default();
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(Rect::new(0, 0, 80, 24));
    for (width, height, code) in [
        (80, 24, KeyCode::Down),
        (80, 24, KeyCode::Enter),
        (80, 24, KeyCode::F(1)),
        (80, 24, KeyCode::Esc),
        (72, 20, KeyCode::Down),
        (120, 32, KeyCode::Up),
        (120, 32, KeyCode::F(1)),
        (120, 32, KeyCode::Esc),
    ] {
        crate::input::dispatch_event(
            &mut state,
            &Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
            None,
            None,
        );
        let area = Rect::new(0, 0, width, height);
        super::super::refresh_scroll_layout(&mut state, area, None);
        let full = super::super::render_shell_buffer(area, &state);
        draw(&mut renderer, &mut screen, &full);
    }
}

#[test]
fn mixed_unicode_and_styles_replay_across_many_frames_without_full_clears() {
    let labels = ["日本語", "Tiếng Việt", "a\u{0301}", "👩‍💻", "x", "", "   "];
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(Rect::new(0, 0, 30, 4));
    for tick in 0..160_usize {
        let (width, height) = if (tick / 20) % 2 == 0 {
            (30, 4)
        } else {
            (21, 6)
        };
        let rows: Vec<_> = (0..height)
            .map(|row| labels[(tick + usize::from(row)) % labels.len()].repeat(tick % 4 + 1))
            .collect();
        let mut full = buffer(width, height, &rows.join("\n"));
        let row = u16::try_from(tick % usize::from(height)).unwrap();
        full.set_style(
            Rect::new(0, row, width, 1),
            Style::new()
                .fg(Color::LightCyan)
                .bg(Color::Indexed(17))
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );
        let output = draw(&mut renderer, &mut screen, &full);
        let full_clear = output.bytes.windows(4).any(|value| value == b"\x1b[2J");
        assert_eq!(full_clear, tick % 20 == 0);
        assert_eq!((output.writes, output.flushes), (1, 1));
    }
}

#[test]
fn vs16_emoji_and_unicode_pairs_preserve_the_new_wide_glyph() {
    // B F01: the old diff wrote the heart, then a space at its continuation
    // column. The independent terminal replay must retain both expected hearts.
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(Rect::new(0, 0, 24, 3));
    draw(&mut renderer, &mut screen, &buffer(24, 3, "AB          ❤️"));
    let output = draw(&mut renderer, &mut screen, &buffer(24, 3, "❤️          ❤️"));
    assert!(!output.bytes.windows(4).any(|seq| seq == b"\x1b[2J"));
    let labels = [
        "日本語",
        "Tiếng Việt a\u{0301}",
        "🇯🇵",
        "👨‍👩‍👧‍👦",
        "❤️",
        "✈️",
        "☀️",
        "x",
        "",
        "long tail 1234",
    ];
    for (previous_index, previous) in labels.iter().enumerate() {
        for (next_index, next) in labels.iter().enumerate() {
            let mut renderer = FrameRenderer::default();
            let mut screen = Screen::new(Rect::new(0, 0, 24, 3));
            let mut old = buffer(24, 3, previous);
            old.set_style(
                Rect::new(0, 0, 24, 1),
                Style::new()
                    .bg(Color::Indexed(20))
                    .add_modifier(Modifier::UNDERLINED | Modifier::REVERSED),
            );
            draw(&mut renderer, &mut screen, &old);
            let mut new = buffer(24, 3, next);
            if (previous_index + next_index) % 2 == 0 {
                new.set_style(
                    Rect::new(0, 0, 24, 1),
                    Style::new()
                        .fg(Color::Rgb(12, 90, 31))
                        .add_modifier(Modifier::ITALIC | Modifier::BOLD),
                );
            }
            let output = draw(&mut renderer, &mut screen, &new);
            assert!(!output.bytes.windows(4).any(|seq| seq == b"\x1b[2J"));
        }
    }
}

#[cfg(windows)]
#[test]
fn ansi_recorder_works_without_console() {
    if std::env::var_os("NT_T14_DETACHED_FRAME_TEST").is_some() {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn FreeConsole() -> i32;
        }
        // Child only: no other test runner loses its console or changes env.
        unsafe {
            FreeConsole();
        }
        assert!(!crossterm::ansi_support::supports_ansi());
        let mut capture = Capture::default();
        let mut full = buffer(20, 3, "日本 Tiếng Việt");
        full.set_style(
            Rect::new(0, 0, 20, 1),
            Style::new()
                .fg(Color::Rgb(12, 90, 31))
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );
        FrameRenderer::default().draw(&mut capture, &full).unwrap();
        assert!(capture.bytes.starts_with(b"\x1b[2J"));
        assert_eq!((capture.writes, capture.flushes), (1, 1));
        let mut screen = Screen::new(full.area);
        screen.replay(&capture.bytes);
        screen.assert_matches(&full);
        return;
    }
    for no_color in [Some("1"), None] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "terminal::frame::tests::ansi_recorder_works_without_console",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("NT_T14_DETACHED_FRAME_TEST", "1")
            .env("TERM", "dumb");
        if let Some(value) = no_color {
            command.env("NO_COLOR", value);
        } else {
            command.env_remove("NO_COLOR");
        }
        let child = command.output().unwrap();
        assert!(
            child.status.success(),
            "Detached child NO_COLOR={no_color:?} failed: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    }
}

#[test]
fn caret_changes_share_the_frame_batch_and_do_not_skip_identical_cells() {
    let full = buffer(24, 3, "Tiếng Việt 日本");
    let mut renderer = FrameRenderer::default();
    let mut screen = Screen::new(full.area);
    for caret in [Some((4, 1)), Some((5, 1)), None] {
        let mut output = Capture::default();
        renderer
            .draw_with_cursor(&mut output, &full, caret)
            .unwrap();
        assert_eq!((output.writes, output.flushes), (1, 1));
        screen.replay(&output.bytes);
        screen.assert_matches(&full);
        assert_eq!(screen.cursor_visible, caret.is_some());
        if let Some((x, y)) = caret {
            assert_eq!((screen.x, screen.y), (usize::from(x), usize::from(y)));
        }
        let mut identical = Capture::default();
        renderer
            .draw_with_cursor(&mut identical, &full, caret)
            .unwrap();
        assert_eq!((identical.writes, identical.flushes), (0, 0));
    }
}

#[test]
fn t15_form_focus_help_and_resize_replay_cells_and_the_actual_caret() {
    use crate::rendering::input_form::{self, InputControl, InputForm};
    let mut state = crate::state::AppState::default();
    state.navigate_to(crate::navigation::Screen::VolumesAndChapters);
    state.set_workspace_controls(true);
    state.focus_workspace();
    let mut renderer = FrameRenderer::default();
    for (width, height) in [(80, 24), (120, 32), (134, 35), (80, 24)] {
        let area = Rect::new(0, 0, width, height);
        let mut screen = Screen::new(area);
        renderer.invalidate();
        for (focus, help, text) in [
            (
                InputControl::Text,
                false,
                "日本語 Tiếng Việt a\u{301}".repeat(12),
            ),
            (InputControl::Primary, false, "短".into()),
            (InputControl::Text, true, "短".into()),
            (InputControl::Text, false, "短".into()),
        ] {
            if help {
                state.open_help();
            } else {
                state.close_help();
            }
            let form = InputForm {
                title: "Đổi tên chương",
                label: "Tên chương mới:",
                text: Some(text),
                message: String::new(),
                primary: "Xem trước",
                focused: focus,
            };
            let mut full = Buffer::empty(area);
            crate::rendering::render(&mut full, area, &state);
            let caret = input_form::render(&mut full, area, &state, &form);
            let mut capture = Capture::default();
            renderer
                .draw_with_cursor(&mut capture, &full, caret)
                .unwrap();
            screen.replay(&capture.bytes);
            screen.assert_matches(&full);
            assert_eq!(screen.cursor_visible, focus == InputControl::Text && !help);
            if let Some((x, y)) = caret {
                assert_eq!((screen.x, screen.y), (usize::from(x), usize::from(y)));
            }
        }
    }
}
