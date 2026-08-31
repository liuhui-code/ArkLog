use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthChar;

use crate::{theme, TextInputView};

pub(crate) fn render_text_input(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    input: TextInputView<'_>,
    color: Color,
) {
    let width = area.width.saturating_sub(2) as usize;
    let (visible, cursor_column) = visible_window(input, width);
    frame.render_widget(
        Paragraph::new(styled_input(visible, color)).block(theme::panel(title)),
        area,
    );
    let cursor_x = area
        .x
        .saturating_add(1 + cursor_column.min(width.saturating_sub(1)) as u16);
    frame.set_cursor_position((cursor_x, area.y + 1));
}

fn visible_window(input: TextInputView<'_>, width: usize) -> (TextInputView<'_>, usize) {
    let cursor_limit = width.saturating_sub(1);
    let mut start = input.cursor;
    let mut cursor_column = 0;
    for (index, character) in input.text[..input.cursor].char_indices().rev() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if cursor_column + character_width > cursor_limit {
            break;
        }
        cursor_column += character_width;
        start = index;
    }

    let mut end = start;
    let mut used = 0;
    for (offset, character) in input.text[start..].char_indices() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > width {
            break;
        }
        used += character_width;
        end = start + offset + character.len_utf8();
    }
    let selection = input
        .selection
        .and_then(|(selection_start, selection_end)| {
            let selection_start = selection_start.max(start);
            let selection_end = selection_end.min(end);
            (selection_start < selection_end)
                .then_some((selection_start - start, selection_end - start))
        });
    (
        TextInputView {
            text: &input.text[start..end],
            cursor: input.cursor - start,
            selection,
        },
        cursor_column,
    )
}

fn styled_input(input: TextInputView<'_>, color: Color) -> Line<'_> {
    let Some((start, end)) = input.selection else {
        return Line::from(Span::styled(input.text, Style::new().fg(color)));
    };
    Line::from(vec![
        Span::styled(&input.text[..start], Style::new().fg(color)),
        Span::styled(&input.text[start..end], theme::input_selection(color)),
        Span::styled(&input.text[end..], Style::new().fg(color)),
    ])
}
