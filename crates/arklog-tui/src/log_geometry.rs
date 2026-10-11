use std::collections::VecDeque;
use std::ops::Range;

use regex::RegexBuilder;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme;

pub(crate) struct VisibleGrapheme {
    pub(crate) source: Range<usize>,
    pub(crate) text: String,
}

pub(crate) struct HorizontalSlice {
    pub(crate) graphemes: Vec<VisibleGrapheme>,
    pub(crate) hidden_left: bool,
    pub(crate) hidden_right: bool,
}

pub(crate) struct WrappedSlice {
    pub(crate) graphemes: Vec<VisibleGrapheme>,
    pub(crate) continuation: bool,
}

pub(crate) fn display_width(text: &str) -> u64 {
    let mut column = 0;
    for grapheme in text.graphemes(true) {
        column += grapheme_width(grapheme, column);
    }
    column
}

pub(crate) fn horizontal_slice(text: &str, offset: u64, width: usize) -> HorizontalSlice {
    let width = width as u64;
    let total_width = display_width(text);
    let hidden_left = offset > 0 && total_width > 0;
    let left_marker_width = if hidden_left {
        theme::OVERFLOW_MARKER_WIDTH_CELLS
    } else {
        0
    };
    let width_without_left = width.saturating_sub(left_marker_width);
    let hidden_right = total_width > offset.saturating_add(width_without_left);
    let right_marker_width = if hidden_right {
        theme::OVERFLOW_MARKER_WIDTH_CELLS
    } else {
        0
    };
    let payload_width = width_without_left.saturating_sub(right_marker_width);
    let payload_end = offset.saturating_add(payload_width);
    let mut graphemes = Vec::new();
    let mut column = 0;
    for (start, grapheme) in text.grapheme_indices(true) {
        let grapheme_width = grapheme_width(grapheme, column);
        let end_column = column.saturating_add(grapheme_width);
        if column >= offset && end_column <= payload_end {
            graphemes.push(VisibleGrapheme {
                source: start..start + grapheme.len(),
                text: if grapheme == "\t" {
                    " ".repeat(grapheme_width as usize)
                } else {
                    grapheme.to_string()
                },
            });
        }
        if column >= payload_end || end_column > payload_end {
            break;
        }
        column = end_column;
    }
    HorizontalSlice {
        graphemes,
        hidden_left,
        hidden_right,
    }
}

pub(crate) fn first_literal_match_range(text: &str, query: &str) -> Option<Range<usize>> {
    literal_pattern(query)?
        .find(text)
        .map(|found| found.range())
}

pub(crate) fn literal_match_ranges_in(
    text: &str,
    query: &str,
    visible: &Range<usize>,
) -> Vec<Range<usize>> {
    let Some(pattern) = literal_pattern(query) else {
        return Vec::new();
    };
    pattern
        .find_iter(text)
        .skip_while(|found| found.end() <= visible.start)
        .take_while(|found| found.start() < visible.end)
        .filter(|found| !found.is_empty())
        .map(|found| found.range())
        .collect()
}

fn literal_pattern(query: &str) -> Option<regex::Regex> {
    if query.is_empty() {
        return None;
    }
    RegexBuilder::new(&regex::escape(query))
        .case_insensitive(true)
        .build()
        .ok()
}

pub(crate) fn cell_range_for_bytes(text: &str, bytes: &Range<usize>) -> Option<Range<u64>> {
    let mut column = 0;
    let mut start_column = None;
    let mut end_column = None;
    for (start, grapheme) in text.grapheme_indices(true) {
        let width = grapheme_width(grapheme, column);
        let end = start + grapheme.len();
        if start < bytes.end && end > bytes.start {
            start_column.get_or_insert(column);
            end_column = Some(column.saturating_add(width));
        }
        column = column.saturating_add(width);
        if start >= bytes.end {
            break;
        }
    }
    start_column.zip(end_column).map(|(start, end)| start..end)
}

fn grapheme_width(grapheme: &str, column: u64) -> u64 {
    if grapheme == "\t" {
        theme::LOG_TAB_STOP_CELLS - column % theme::LOG_TAB_STOP_CELLS
    } else {
        UnicodeWidthStr::width(grapheme) as u64
    }
}

pub(crate) fn wrapped_row_starts_from(
    text: &str,
    start_cell: u64,
    width: usize,
    limit: usize,
) -> Vec<u64> {
    if limit == 0 {
        return Vec::new();
    }
    let width = (width as u64).max(theme::MIN_LOG_CONTENT_WIDTH_CELLS);
    let mut starts = vec![start_cell];
    if text.is_empty() {
        return starts;
    }
    let mut column: u64 = 0;
    let mut used: u64 = 0;
    let mut capacity = wrapped_payload_width(width, start_cell > 0);
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme, column);
        if column < start_cell {
            column = column.saturating_add(grapheme_width);
            continue;
        }
        if used > 0 && used.saturating_add(grapheme_width) > capacity {
            if starts.len() == limit {
                break;
            }
            starts.push(column);
            used = 0;
            capacity = wrapped_payload_width(width, true);
        }
        used = used.saturating_add(grapheme_width);
        column = column.saturating_add(grapheme_width);
    }
    starts
}

pub(crate) fn wrapped_slices(
    text: &str,
    start_cell: u64,
    width: usize,
    limit: usize,
) -> Vec<WrappedSlice> {
    if limit == 0 {
        return Vec::new();
    }
    let width = (width as u64).max(theme::MIN_LOG_CONTENT_WIDTH_CELLS);
    let mut rows = Vec::with_capacity(limit);
    let mut row = WrappedSlice {
        graphemes: Vec::new(),
        continuation: start_cell > 0,
    };
    let mut column: u64 = 0;
    let mut used: u64 = 0;
    let mut capacity = wrapped_payload_width(width, row.continuation);
    for (start, grapheme) in text.grapheme_indices(true) {
        let grapheme_width = grapheme_width(grapheme, column);
        if column < start_cell {
            column = column.saturating_add(grapheme_width);
            continue;
        }
        if used > 0 && used.saturating_add(grapheme_width) > capacity {
            rows.push(row);
            if rows.len() == limit {
                return rows;
            }
            row = WrappedSlice {
                graphemes: Vec::new(),
                continuation: true,
            };
            used = 0;
            capacity = wrapped_payload_width(width, true);
        }
        row.graphemes.push(VisibleGrapheme {
            source: start..start + grapheme.len(),
            text: if grapheme == "\t" {
                " ".repeat(grapheme_width as usize)
            } else {
                grapheme.to_string()
            },
        });
        used = used.saturating_add(grapheme_width);
        column = column.saturating_add(grapheme_width);
    }
    if !row.graphemes.is_empty() || (text.is_empty() && start_cell == 0) {
        rows.push(row);
    }
    rows
}

pub(crate) fn wrapped_row_starts_tail(text: &str, width: usize, limit: usize) -> Vec<u64> {
    if limit == 0 {
        return Vec::new();
    }
    let width = (width as u64).max(theme::MIN_LOG_CONTENT_WIDTH_CELLS);
    let mut tail = VecDeque::with_capacity(limit);
    tail.push_back(0);
    let mut column: u64 = 0;
    let mut used: u64 = 0;
    let mut capacity = wrapped_payload_width(width, false);
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme, column);
        if used > 0 && used.saturating_add(grapheme_width) > capacity {
            if tail.len() == limit {
                tail.pop_front();
            }
            tail.push_back(column);
            used = 0;
            capacity = wrapped_payload_width(width, true);
        }
        used = used.saturating_add(grapheme_width);
        column = column.saturating_add(grapheme_width);
    }
    tail.into_iter().collect()
}

pub(crate) fn previous_wrapped_row_start(text: &str, before: u64, width: usize) -> Option<u64> {
    if before == 0 {
        return None;
    }
    let width = (width as u64).max(theme::MIN_LOG_CONTENT_WIDTH_CELLS);
    let mut previous = Some(0);
    let mut column: u64 = 0;
    let mut used: u64 = 0;
    let mut capacity = wrapped_payload_width(width, false);
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme, column);
        if used > 0 && used.saturating_add(grapheme_width) > capacity {
            if column >= before {
                break;
            }
            previous = Some(column);
            used = 0;
            capacity = wrapped_payload_width(width, true);
        }
        used = used.saturating_add(grapheme_width);
        column = column.saturating_add(grapheme_width);
    }
    previous.filter(|start| *start < before)
}

pub(crate) fn next_wrapped_row_start(text: &str, current: u64, width: usize) -> Option<u64> {
    wrapped_row_starts_from(text, current, width, 2)
        .get(1)
        .copied()
}

fn wrapped_payload_width(width: u64, continuation: bool) -> u64 {
    if continuation {
        width
            .saturating_sub(theme::WRAPPED_CONTINUATION_WIDTH_CELLS)
            .max(theme::MIN_LOG_CONTENT_WIDTH_CELLS)
    } else {
        width
    }
}

#[cfg(test)]
mod tests {
    use super::{wrapped_row_starts_tail, wrapped_slices};

    #[test]
    fn huge_records_only_materialize_the_requested_wrapped_rows() {
        let huge = "x".repeat(1_000_000);
        let tail = wrapped_row_starts_tail(&huge, 80, 3);
        assert_eq!(tail.len(), 3);
        assert!(tail[0] > u16::MAX as u64);

        let unicode = format!("{}中e\u{301}👩‍💻\tTAIL", "x".repeat(100_000));
        let rows = wrapped_slices(&unicode, 99_990, 8, 3);
        assert!(rows.len() <= 3);
        let visible = rows
            .iter()
            .flat_map(|row| row.graphemes.iter())
            .map(|grapheme| grapheme.text.as_str())
            .collect::<String>();
        assert!(visible.contains('中'));
        assert!(visible.contains("e\u{301}"));
        assert!(visible.contains("👩‍💻"));
    }
}
