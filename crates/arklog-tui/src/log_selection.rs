use std::ops::Range;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::{Position as ScreenPosition, Rect},
};
use unicode_segmentation::UnicodeSegmentation;

use super::{ArkLogState, ViewportAnchor};
use crate::{
    log_geometry::{
        display_width, grapheme_width, horizontal_slice, wrapped_row_starts_tail, wrapped_slices,
        VisibleGrapheme,
    },
    theme, Clipboard,
};

const MAX_COPY_RECORDS: u64 = 10_000;
const MAX_COPY_BYTES: usize = 1024 * 1024;

fn byte_at_cell(text: &str, cell: u64) -> usize {
    let mut column = 0;
    for (byte, grapheme) in text.grapheme_indices(true) {
        let end = column + grapheme_width(grapheme, column);
        if end > cell {
            return byte;
        }
        column = end;
    }
    text.len()
}

fn byte_at_screen_column(graphemes: &[VisibleGrapheme], column: u64) -> usize {
    let mut cell = 0;
    for grapheme in graphemes {
        cell += display_width(&grapheme.text);
        if column < cell {
            return grapheme.source.start;
        }
    }
    graphemes.last().map_or(0, |grapheme| grapheme.source.end)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Position {
    record: u64,
    byte: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct LogSelection {
    anchor: Position,
    head: Position,
}

impl LogSelection {
    pub fn range_for_record(&self, record: u64, length: usize) -> Range<usize> {
        let (start, end) = self.ordered();
        if record < start.record || record > end.record {
            return 0..0;
        }
        let from = if record == start.record {
            start.byte
        } else {
            0
        };
        let to = if record == end.record {
            end.byte
        } else {
            length
        };
        from.min(length)..to.min(length)
    }

    fn ordered(&self) -> (Position, Position) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }
}

impl ArkLogState {
    pub fn log_selection(&self) -> Option<&LogSelection> {
        self.selection.as_ref()
    }

    pub fn clear_log_selection(&mut self) {
        self.selection = None;
        self.mouse_dragging = false;
    }

    pub fn handle_log_mouse(&mut self, mouse: MouseEvent, content: Rect) -> std::io::Result<bool> {
        if content.is_empty() {
            return Ok(false);
        }
        let inside = content.contains(ScreenPosition::new(mouse.column, mouse.row));
        match mouse.kind {
            MouseEventKind::ScrollUp if inside => {
                self.scroll_up(theme::MOUSE_SCROLL_ROWS, self.viewport_height)?;
                Ok(true)
            }
            MouseEventKind::ScrollDown if inside => {
                self.scroll_down(theme::MOUSE_SCROLL_ROWS, self.viewport_height)?;
                Ok(true)
            }
            MouseEventKind::Down(MouseButton::Left) if inside => {
                let Some(position) = self.log_position_at(mouse, content)? else {
                    return Ok(false);
                };
                let anchor = self.current_display_anchor()?;
                let selection_anchor = if mouse.modifiers == KeyModifiers::SHIFT {
                    self.selection
                        .map_or(position, |selection| selection.anchor)
                } else {
                    position
                };
                self.selection = Some(LogSelection {
                    anchor: selection_anchor,
                    head: position,
                });
                self.viewport_anchor = Some(anchor);
                self.mouse_dragging = true;
                Ok(true)
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
                if self.mouse_dragging =>
            {
                if let Some(position) = self.log_position_at(mouse, content)? {
                    if let Some(selection) = &mut self.selection {
                        selection.head = position;
                    }
                }
                if matches!(mouse.kind, MouseEventKind::Up(_)) {
                    self.mouse_dragging = false;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn log_position_at(
        &mut self,
        mouse: MouseEvent,
        content: Rect,
    ) -> std::io::Result<Option<Position>> {
        let target_row = mouse.row.clamp(content.y, content.bottom() - 1) - content.y;
        let column = mouse.column.clamp(content.x, content.right()) - content.x;
        let window = self.display_window()?;
        let mut row = 0;
        let mut last = None;
        for (offset, text) in window.records.iter().enumerate() {
            let record = window.start_record + offset as u64;
            if self.soft_wrap {
                let start_cell = if offset == 0 { window.start_cell } else { 0 };
                for slice in wrapped_slices(
                    text,
                    start_cell,
                    content.width as usize,
                    self.viewport_height.saturating_sub(row),
                ) {
                    let marker = if slice.continuation {
                        theme::WRAPPED_CONTINUATION_WIDTH_CELLS
                    } else {
                        0
                    };
                    let byte = byte_at_screen_column(
                        &slice.graphemes,
                        u64::from(column).saturating_sub(marker),
                    );
                    last = Some(Position { record, byte });
                    if row == usize::from(target_row) {
                        return Ok(last);
                    }
                    row += 1;
                }
            } else {
                let slice = horizontal_slice(text, self.horizontal_offset, content.width as usize);
                let marker = if slice.hidden_left {
                    theme::OVERFLOW_MARKER_WIDTH_CELLS
                } else {
                    0
                };
                let byte = byte_at_screen_column(
                    &slice.graphemes,
                    u64::from(column).saturating_sub(marker),
                );
                last = Some(Position { record, byte });
                if row == usize::from(target_row) {
                    return Ok(last);
                }
                row += 1;
            }
        }
        Ok(last)
    }

    pub fn handle_log_key(
        &mut self,
        key: KeyEvent,
        clipboard: &mut impl Clipboard,
    ) -> Result<bool, String> {
        if matches!(key.code, KeyCode::Char('a' | 'A'))
            && [KeyModifiers::CONTROL, KeyModifiers::SUPER].contains(&key.modifiers)
        {
            if self.visible_count() == 0 {
                return Ok(true);
            }
            let anchor = self.current_display_anchor().map_err(|e| e.to_string())?;
            let record = self.visible_count() - 1;
            let byte = self
                .visible_record(record)
                .map_err(|e| e.to_string())?
                .unwrap_or_default()
                .len();
            self.selection = Some(LogSelection {
                anchor: Position { record: 0, byte: 0 },
                head: Position { record, byte },
            });
            self.viewport_anchor = Some(anchor);
            return Ok(true);
        }
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            let selected = self.selection.is_some();
            self.clear_log_selection();
            return Ok(selected);
        }
        if matches!(key.code, KeyCode::Char('c' | 'C'))
            && [
                KeyModifiers::CONTROL,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                KeyModifiers::SUPER,
            ]
            .contains(&key.modifiers)
        {
            if let Some(text) = self
                .selected_log_text()
                .map_err(|error| error.to_string())?
            {
                clipboard.set_text(&text)?;
                return Ok(true);
            }
            return Ok(false);
        }
        if key.modifiers != KeyModifiers::SHIFT
            || !matches!(
                key.code,
                KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::PageUp
                    | KeyCode::PageDown
            )
            || self.visible_count() == 0
        {
            if key.modifiers.is_empty()
                && matches!(
                    key.code,
                    KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Home
                        | KeyCode::End
                        | KeyCode::PageUp
                        | KeyCode::PageDown
                )
            {
                self.clear_log_selection();
            }
            return Ok(false);
        }
        if self.selection.is_none() {
            let window = self.display_window().map_err(|error| error.to_string())?;
            let cell = if self.soft_wrap {
                window.start_cell
            } else {
                self.horizontal_offset
            };
            let position = Position {
                record: window.start_record,
                byte: byte_at_cell(&window.records[0], cell),
            };
            self.selection = Some(LogSelection {
                anchor: position,
                head: position,
            });
            self.viewport_anchor = Some(ViewportAnchor {
                record: window.start_record,
                cell: window.start_cell,
            });
        }
        let mut selection = self.selection.unwrap();
        let line = self
            .visible_record(selection.head.record)
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        match key.code {
            KeyCode::Right => {
                if let Some(grapheme) = line[selection.head.byte..].graphemes(true).next() {
                    selection.head.byte += grapheme.len();
                } else if selection.head.record + 1 < self.visible_count() {
                    selection.head = Position {
                        record: selection.head.record + 1,
                        byte: 0,
                    };
                }
            }
            KeyCode::Left => {
                if let Some((byte, _)) = line[..selection.head.byte]
                    .grapheme_indices(true)
                    .next_back()
                {
                    selection.head.byte = byte;
                } else if selection.head.record > 0 {
                    selection.head.record -= 1;
                    selection.head.byte = self
                        .visible_record(selection.head.record)
                        .map_err(|e| e.to_string())?
                        .unwrap_or_default()
                        .len();
                }
            }
            KeyCode::Home => selection.head.byte = 0,
            KeyCode::End => selection.head.byte = line.len(),
            code => {
                let rows = if matches!(code, KeyCode::PageUp | KeyCode::PageDown) {
                    self.viewport_height as u64
                } else {
                    1
                };
                let record = if matches!(code, KeyCode::Up | KeyCode::PageUp) {
                    selection.head.record.saturating_sub(rows)
                } else {
                    (selection.head.record + rows).min(self.visible_count() - 1)
                };
                let column = display_width(&line[..selection.head.byte]);
                let next = self
                    .visible_record(record)
                    .map_err(|e| e.to_string())?
                    .unwrap_or_default();
                selection.head = Position {
                    record,
                    byte: byte_at_cell(&next, column),
                };
            }
        }
        self.selection = Some(selection);
        self.reveal_selection_head().map_err(|e| e.to_string())?;
        Ok(true)
    }

    fn reveal_selection_head(&mut self) -> std::io::Result<()> {
        let Some(selection) = self.selection else {
            return Ok(());
        };
        let text = self
            .visible_record(selection.head.record)?
            .unwrap_or_default();
        let column = display_width(&text[..selection.head.byte]);
        if self.soft_wrap {
            let window = self.display_window()?;
            let mut remaining = self.viewport_height;
            let mut visible = false;
            for (offset, text) in window.records.iter().enumerate() {
                let start_cell = if offset == 0 { window.start_cell } else { 0 };
                let rows = wrapped_slices(text, start_cell, self.viewport_width, remaining);
                remaining = remaining.saturating_sub(rows.len());
                if window.start_record + offset as u64 == selection.head.record {
                    let first = rows.iter().flat_map(|row| &row.graphemes).next();
                    let last = rows.iter().flat_map(|row| &row.graphemes).last();
                    visible = first.zip(last).is_some_and(|(first, last)| {
                        selection.head.byte >= first.source.start
                            && selection.head.byte <= last.source.end
                    });
                }
            }
            if !visible {
                let starts = wrapped_row_starts_tail(
                    &text[..selection.head.byte],
                    self.viewport_width,
                    self.viewport_height,
                );
                self.viewport_anchor = Some(ViewportAnchor {
                    record: selection.head.record,
                    cell: starts.first().copied().unwrap_or(0),
                });
            }
            return Ok(());
        }
        let start = self.visible_window_start(self.viewport_height);
        if selection.head.record < start {
            self.viewport_anchor = Some(ViewportAnchor {
                record: selection.head.record,
                cell: 0,
            });
        } else if selection.head.record >= start + self.viewport_height as u64 {
            self.viewport_anchor = Some(ViewportAnchor {
                record: selection.head.record + 1 - self.viewport_height as u64,
                cell: 0,
            });
        }
        let available = (self.viewport_width as u64)
            .saturating_sub(theme::HORIZONTAL_MATCH_MARKER_RESERVE_CELLS)
            .max(1);
        if column < self.horizontal_offset {
            self.horizontal_offset = column;
        } else if column > self.horizontal_offset.saturating_add(available) {
            let byte = byte_at_cell(&text, column.saturating_sub(available));
            self.horizontal_offset = display_width(&text[..byte]);
        }
        Ok(())
    }

    pub fn selected_log_text(&mut self) -> std::io::Result<Option<String>> {
        let Some(selection) = self
            .selection
            .filter(|selection| selection.anchor != selection.head)
        else {
            return Ok(None);
        };
        let (start, end) = selection.ordered();
        if end.record - start.record >= MAX_COPY_RECORDS {
            return Err(std::io::Error::other(
                "copy exceeds 10,000 records; select a smaller range",
            ));
        }
        let mut text = String::new();
        for record in start.record..=end.record {
            let line = self.visible_record(record)?.unwrap_or_default();
            let range = selection.range_for_record(record, line.len());
            if text.len() + range.len() + usize::from(record > start.record) > MAX_COPY_BYTES {
                return Err(std::io::Error::other(
                    "copy exceeds 1 MiB; select a smaller range",
                ));
            }
            if record > start.record {
                text.push('\n');
            }
            text.push_str(&line[range]);
        }
        Ok(Some(text))
    }
}
