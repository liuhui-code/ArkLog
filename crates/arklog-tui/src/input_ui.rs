use ratatui::{
    layout::Rect,
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
    tone: theme::Tone,
) {
    let width = theme::input_content_width(area);
    let (visible, cursor_column) = visible_window(input, width);
    frame.render_widget(
        Paragraph::new(styled_input(visible, tone)).block(theme::panel(title)),
        area,
    );
    frame.set_cursor_position(theme::input_cursor_position(area, cursor_column, width));
}

fn visible_window(input: TextInputView<'_>, width: usize) -> (TextInputView<'_>, usize) {
    let cursor_limit = theme::input_cursor_limit(width);
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

fn styled_input(input: TextInputView<'_>, tone: theme::Tone) -> Line<'_> {
    let Some((start, end)) = input.selection else {
        return Line::from(Span::styled(input.text, theme::text(tone)));
    };
    Line::from(vec![
        Span::styled(&input.text[..start], theme::text(tone)),
        Span::styled(&input.text[start..end], theme::input_selection(tone)),
        Span::styled(&input.text[end..], theme::text(tone)),
    ])
}
