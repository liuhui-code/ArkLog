use crate::{
    log_geometry::{horizontal_slice, literal_match_ranges_in, wrapped_slices, VisibleGrapheme},
    theme,
};
use ratatui::text::{Line, Span};
use regex::Regex;

pub(super) fn styled_log_line(
    raw: &str,
    filter: Option<&Regex>,
    find_query: &str,
    current_find: bool,
    horizontal_offset: u64,
    content_width: usize,
    selected: &std::ops::Range<usize>,
) -> Line<'static> {
    let level = log_level_field(raw);
    let visible = horizontal_slice(raw, horizontal_offset, content_width);
    let visible_bytes = visible_byte_range(&visible.graphemes);
    let filter_ranges = visible_bytes
        .as_ref()
        .map_or_else(Vec::new, |range| regex_match_ranges_in(raw, filter, range));
    let find_ranges = visible_bytes.as_ref().map_or_else(Vec::new, |range| {
        literal_match_ranges_in(raw, find_query, range)
    });
    let mut spans = Vec::new();
    if visible.hidden_left {
        spans.push(Span::styled(
            theme::LEFT_OVERFLOW_MARKER,
            theme::overflow_marker(current_find),
        ));
    }
    spans.extend(styled_graphemes(
        visible.graphemes,
        level.as_ref(),
        &filter_ranges,
        &find_ranges,
        current_find,
        selected,
    ));
    if visible.hidden_right {
        spans.push(Span::styled(
            theme::RIGHT_OVERFLOW_MARKER,
            theme::overflow_marker(current_find),
        ));
    }
    Line::from(spans)
}

pub(super) fn styled_wrapped_log_lines(
    raw: &str,
    filter: Option<&Regex>,
    find_query: &str,
    current_find: bool,
    start_cell: u64,
    content_width: usize,
    limit: usize,
    selected: &std::ops::Range<usize>,
) -> Vec<Line<'static>> {
    let level = log_level_field(raw);
    let rows = wrapped_slices(raw, start_cell, content_width, limit);
    let visible_bytes = rows.iter().flat_map(|row| row.graphemes.iter()).fold(
        None,
        |range: Option<std::ops::Range<usize>>, grapheme| {
            Some(match range {
                Some(range) => {
                    range.start.min(grapheme.source.start)..range.end.max(grapheme.source.end)
                }
                None => grapheme.source.clone(),
            })
        },
    );
    let filter_ranges = visible_bytes
        .as_ref()
        .map_or_else(Vec::new, |range| regex_match_ranges_in(raw, filter, range));
    let find_ranges = visible_bytes.as_ref().map_or_else(Vec::new, |range| {
        literal_match_ranges_in(raw, find_query, range)
    });
    rows.into_iter()
        .map(|row| {
            let mut spans = Vec::new();
            if row.continuation {
                spans.push(Span::styled(
                    theme::WRAPPED_CONTINUATION_MARKER,
                    theme::wrapped_continuation(current_find),
                ));
            }
            spans.extend(styled_graphemes(
                row.graphemes,
                level.as_ref(),
                &filter_ranges,
                &find_ranges,
                current_find,
                selected,
            ));
            Line::from(spans)
        })
        .collect()
}

fn visible_byte_range(graphemes: &[VisibleGrapheme]) -> Option<std::ops::Range<usize>> {
    let first = graphemes.first()?;
    let last = graphemes.last()?;
    Some(first.source.start..last.source.end)
}

fn regex_match_ranges_in(
    raw: &str,
    filter: Option<&Regex>,
    visible: &std::ops::Range<usize>,
) -> Vec<std::ops::Range<usize>> {
    filter
        .into_iter()
        .flat_map(|filter| filter.find_iter(raw))
        .skip_while(|found| found.end() <= visible.start)
        .take_while(|found| found.start() < visible.end)
        .filter(|found| !found.is_empty())
        .map(|found| found.range())
        .collect()
}

fn styled_graphemes(
    graphemes: Vec<VisibleGrapheme>,
    level: Option<&(std::ops::Range<usize>, theme::LogLevel)>,
    filter_ranges: &[std::ops::Range<usize>],
    find_ranges: &[std::ops::Range<usize>],
    current_find: bool,
    selected: &std::ops::Range<usize>,
) -> Vec<Span<'static>> {
    graphemes
        .into_iter()
        .map(|grapheme| {
            let span_level = level
                .filter(|(range, _)| ranges_overlap(range, &grapheme.source))
                .map(|(_, level)| *level);
            let style = theme::log_span(
                span_level,
                current_find,
                find_ranges
                    .iter()
                    .any(|range| ranges_overlap(range, &grapheme.source)),
                filter_ranges
                    .iter()
                    .any(|range| ranges_overlap(range, &grapheme.source)),
                ranges_overlap(selected, &grapheme.source),
            );
            Span::styled(grapheme.text, style)
        })
        .collect()
}

fn ranges_overlap(left: &std::ops::Range<usize>, right: &std::ops::Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn log_level_field(raw: &str) -> Option<(std::ops::Range<usize>, theme::LogLevel)> {
    let fields = raw
        .split_whitespace()
        .take(6)
        .map(|field| {
            let start = field.as_ptr() as usize - raw.as_ptr() as usize;
            (start..start + field.len(), field)
        })
        .collect::<Vec<_>>();
    let standard_start = usize::from(
        fields.len() >= 6
            && fields[0].1.parse::<u64>().is_ok()
            && fields[1].1.contains('-')
            && fields[2].1.contains(':'),
    );
    let candidate = if fields.len() >= standard_start + 5
        && fields[standard_start].1.contains('-')
        && fields[standard_start + 1].1.contains(':')
        && fields[standard_start + 2].1.parse::<u64>().is_ok()
        && fields[standard_start + 3].1.parse::<u64>().is_ok()
    {
        fields.get(standard_start + 4)
    } else if fields.len() >= 4
        && crate::agent_query::parse_epoch_millis(fields[0].1).is_some()
        && fields[1].1.parse::<u64>().is_ok()
        && fields[2].1.parse::<u64>().is_ok()
    {
        fields.get(3)
    } else if fields
        .first()
        .is_some_and(|(_, field)| field.parse::<u64>().is_ok())
    {
        fields.get(1)
    } else {
        None
    }?;
    theme::LogLevel::parse(candidate.1).map(|level| (candidate.0.clone(), level))
}
