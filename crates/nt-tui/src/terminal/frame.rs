use crossterm::Command;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::{
    Attribute, Colored, ContentStyle, Print, SetAttribute, SetAttributes, SetBackgroundColor,
    SetForegroundColor, SetUnderlineColor,
};
use crossterm::terminal::{Clear, ClearType};
use ratatui::backend::IntoCrossterm;
use ratatui::buffer::{Buffer, CellWidth};
use std::io::{self, Write};

#[derive(Default)]
pub(super) struct FrameRenderer {
    previous: Option<Buffer>,
    previous_cursor: Option<Option<(u16, u16)>>,
}

impl FrameRenderer {
    pub(super) fn invalidate(&mut self) {
        self.previous = None;
        self.previous_cursor = None;
    }

    pub(super) fn draw<W: Write>(&mut self, writer: &mut W, next: &Buffer) -> io::Result<()> {
        self.draw_with_cursor(writer, next, None)
    }

    pub(super) fn draw_with_cursor<W: Write>(
        &mut self,
        writer: &mut W,
        next: &Buffer,
        caret: Option<(u16, u16)>,
    ) -> io::Result<()> {
        let caret = caret.filter(|(x, y)| {
            *x >= next.area.x
                && *x < next.area.right()
                && *y >= next.area.y
                && *y < next.area.bottom()
        });
        let previous_cursor = self.previous_cursor.take();
        // Only a completely flushed frame may become the next comparison basis.
        let previous = self.previous.take();
        let empty;
        let prior = match previous.as_ref().filter(|old| old.area == next.area) {
            Some(prior) => prior,
            None => {
                empty = Buffer::empty(next.area);
                &empty
            }
        };
        let full_frame = previous.as_ref().is_none_or(|old| old.area != next.area);
        let changes = prior.diff(next);
        if !full_frame && changes.is_empty() && previous_cursor == Some(caret) {
            self.previous = Some(next.clone());
            self.previous_cursor = Some(caret);
            return Ok(());
        }

        let mut output = String::new();
        if full_frame {
            encode(&mut output, Clear(ClearType::All))?;
        }
        let mut cursor = None;
        let mut last_style = None;
        for (x, y, cell) in changes {
            // A trailing clear emitted for VS16 must not erase the new wide
            // glyph already covering that column (P02-T14-F01).
            if cursor.is_some_and(|(end, row)| row == y && x < end) {
                continue;
            }
            if cursor != Some((x, y)) {
                encode(&mut output, MoveTo(x, y))?;
            }
            let mut style: ContentStyle = cell.style().into_crossterm();
            if Colored::ansi_color_disabled_memoized() {
                // Empty color parameters reset attributes too. Omit colors so
                // selected rows retain their non-color emphasis.
                style.foreground_color = None;
                style.background_color = None;
                style.underline_color = None;
            }
            if last_style != Some(style) {
                encode(&mut output, SetAttribute(Attribute::Reset))?;
                encode_style(&mut output, style)?;
                last_style = Some(style);
            }
            encode(&mut output, Print(cell.symbol()))?;
            cursor = Some((x.saturating_add(cell.symbol().cell_width().max(1)), y));
        }
        encode(&mut output, SetAttribute(Attribute::Reset))?;
        match caret {
            Some((x, y)) => {
                encode(&mut output, MoveTo(x, y))?;
                encode(&mut output, Show)?;
            }
            None => {
                encode(&mut output, MoveTo(next.area.x, next.area.y))?;
                encode(&mut output, Hide)?;
            }
        }
        writer.write_all(output.as_bytes())?;
        writer.flush()?;
        self.previous = Some(next.clone());
        self.previous_cursor = Some(caret);
        Ok(())
    }
}

fn encode_style(output: &mut String, style: ContentStyle) -> io::Result<()> {
    // SetStyle itself dispatches through execute_fmt. Encode primitive commands
    // directly so both colors and attributes stay inside the output batch.
    if let Some(color) = style.background_color {
        encode(output, SetBackgroundColor(color))?;
    }
    if let Some(color) = style.foreground_color {
        encode(output, SetForegroundColor(color))?;
    }
    if let Some(color) = style.underline_color {
        encode(output, SetUnderlineColor(color))?;
    }
    encode(output, SetAttributes(style.attributes))
}

fn encode(output: &mut String, command: impl Command) -> io::Result<()> {
    // Serialize directly, avoiding Windows console side effects before the
    // single batch write, including when the caller is an offline recorder.
    command
        .write_ansi(output)
        .map_err(|_| io::Error::other("terminal command encoding failed"))
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
