use arklog_core::DeviceFaultLogFetchResult;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};
use regex::Regex;

use crate::LogTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Find,
}

pub struct AppView<'a> {
    pub device_id: Option<&'a str>,
    pub device_status: &'a str,
    pub tab: LogTab,
    pub streaming: bool,
    pub runtime_status: &'a str,
    pub raw_count: u64,
    pub visible_count: u64,
    pub following_latest: bool,
    pub filter_query: &'a str,
    pub active_filter: Option<&'a Regex>,
    pub filter_error: Option<&'a str>,
    pub find_query: &'a str,
    pub find_status: (u64, u64),
    pub current_find_visible_index: Option<u64>,
    pub window_start: u64,
    pub lines: &'a [String],
    pub input_mode: InputMode,
    pub input_draft: &'a str,
    pub fault_result: Option<&'a DeviceFaultLogFetchResult>,
    pub selected_fault: usize,
}

pub fn render_app(frame: &mut Frame, view: AppView<'_>) {
    if frame.area().width < 72 || frame.area().height < 16 {
        frame.render_widget(
            Paragraph::new("ArkLog needs at least 72 x 16 terminal cells")
                .block(Block::bordered().title(" TERMINAL TOO SMALL ")),
            frame.area(),
        );
        return;
    }
    let [header, query, workspace, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    render_header(frame, header, &view);
    render_query(frame, query, &view);
    match view.tab {
        LogTab::HiLog => render_hilog(frame, workspace, &view),
        LogTab::FaultLog => render_fault_log(frame, workspace, &view),
    }
    render_footer(frame, footer, &view);
}

fn render_header(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let [device, tabs, stream] = Layout::horizontal([
        Constraint::Length(29),
        Constraint::Fill(1),
        Constraint::Length(30),
    ])
    .areas(area);
    let device_line = match view.device_id {
        Some(id) => Line::from(vec![
            Span::styled("● ", Style::new().fg(Color::Green)),
            Span::styled(id, Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}", view.device_status.to_ascii_uppercase())),
        ]),
        None => Line::styled("No devices", Style::new().fg(Color::Yellow)),
    };
    frame.render_widget(
        Paragraph::new(device_line).block(Block::bordered().title(" DEVICE ")),
        device,
    );
    frame.render_widget(
        Tabs::new([" HiLog ", " Fault Log "])
            .select(usize::from(view.tab == LogTab::FaultLog))
            .divider("│")
            .highlight_style(
                Style::new()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .block(Block::bordered().title(" LOG VIEWS ")),
        tabs,
    );
    let (stream_label, color) = if view.streaming {
        ("● LIVE", Color::Green)
    } else {
        ("■ STOPPED", Color::Yellow)
    };
    let stream_line = if view.device_id.is_none() {
        Line::from("")
    } else {
        Line::from(vec![
            Span::styled(
                stream_label,
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {}", view.runtime_status)),
        ])
    };
    frame.render_widget(
        Paragraph::new(stream_line).block(Block::bordered().title(" STREAM ")),
        stream,
    );
}

fn render_query(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let (title, value, color) = match view.input_mode {
        InputMode::Filter => (
            " REGEX FILTER · ENTER APPLY ",
            view.input_draft,
            Color::Yellow,
        ),
        InputMode::Find => (
            " FIND IN HILOG · ENTER NEXT ",
            view.input_draft,
            Color::Cyan,
        ),
        InputMode::Normal if view.tab == LogTab::FaultLog => (
            " FAULT LOG · R REFRESH ",
            "Inspect raw device diagnostics",
            Color::Gray,
        ),
        InputMode::Normal => (
            " REGEX FILTER · / TO EDIT ",
            if view.filter_query.is_empty() {
                "<all raw logs>"
            } else {
                view.filter_query
            },
            Color::Gray,
        ),
    };
    let value = view.filter_error.unwrap_or(value);
    let color = if view.filter_error.is_some() {
        Color::Red
    } else {
        color
    };
    frame.render_widget(
        Paragraph::new(Span::styled(value, Style::new().fg(color)))
            .block(Block::bordered().title(title)),
        area,
    );
    if view.input_mode != InputMode::Normal {
        let cursor_x = area
            .x
            .saturating_add(1 + view.input_draft.chars().count() as u16);
        frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(2)), area.y + 1));
    }
}

fn render_hilog(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let lines = view
        .lines
        .iter()
        .enumerate()
        .map(|(offset, raw)| {
            let visible_index = view.window_start + offset as u64;
            let current = view.current_find_visible_index == Some(visible_index);
            styled_log_line(raw, view.active_filter, view.find_query, current)
        })
        .collect::<Vec<_>>();
    let title = if view.following_latest {
        " FOLLOWING LATEST "
    } else {
        " SCROLL PAUSED · G TO LATEST "
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        area,
    );
}

fn render_fault_log(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let Some(result) = view.fault_result else {
        frame.render_widget(
            Paragraph::new("Press R to refresh fault diagnostics")
                .block(Block::bordered().title(" FAULT LOG ")),
            area,
        );
        return;
    };
    if result.entries.is_empty() {
        frame.render_widget(
            Paragraph::new(result.message.as_str()).block(Block::bordered().title(" FAULT LOG ")),
            area,
        );
        return;
    }
    let [list, inspector] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Fill(1)]).areas(area);
    let rows = result
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let style = if index == view.selected_fault {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };
            Line::styled(format!("{}  {}", index + 1, entry.id), style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(rows).block(Block::bordered().title(" ENTRIES ")),
        list,
    );
    let raw = &result.entries[view.selected_fault.min(result.entries.len() - 1)].raw;
    frame.render_widget(
        Paragraph::new(raw.as_str()).block(Block::bordered().title(" RAW DIAGNOSTIC ")),
        inspector,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!(
                " {:>7} / {:>7} VISIBLE  │  FIND {}/{}",
                view.visible_count, view.raw_count, view.find_status.0, view.find_status.1
            )),
            Line::styled(
                " Tab view  S start/stop  R refresh  / regex  Ctrl+F find  n/N match  ↑↓/Pg scroll  G latest  C clear  Q quit",
                Style::new().fg(Color::DarkGray),
            ),
        ]),
        area,
    );
}

fn styled_log_line(
    raw: &str,
    filter: Option<&Regex>,
    find_query: &str,
    current_find: bool,
) -> Line<'static> {
    let mut base = if raw.contains(" E ") || raw.contains(" F ") {
        Style::new().fg(Color::Red)
    } else if raw.contains(" W ") {
        Style::new().fg(Color::Yellow)
    } else if raw.contains(" I ") {
        Style::new().fg(Color::Green)
    } else if raw.contains(" D ") {
        Style::new().fg(Color::Blue)
    } else {
        Style::new().fg(Color::Gray)
    };
    if current_find {
        base = base.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
    }
    let filter_ranges = filter
        .into_iter()
        .flat_map(|filter| filter.find_iter(raw))
        .filter(|found| !found.is_empty())
        .map(|found| found.start()..found.end())
        .collect::<Vec<_>>();
    let find_ranges = literal_match_ranges(raw, find_query);
    if filter_ranges.is_empty() && find_ranges.is_empty() {
        return Line::styled(raw.to_string(), base);
    }
    let mut boundaries = vec![0, raw.len()];
    for range in filter_ranges.iter().chain(&find_ranges) {
        boundaries.extend([range.start, range.end]);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut spans = Vec::new();
    for boundary in boundaries.windows(2) {
        let start = boundary[0];
        let end = boundary[1];
        let mut style = base;
        if find_ranges.iter().any(|range| range.contains(&start)) {
            style = style.fg(Color::Cyan).add_modifier(Modifier::UNDERLINED);
        }
        if filter_ranges.iter().any(|range| range.contains(&start)) {
            style = style.fg(Color::Black).bg(Color::Yellow);
        }
        spans.push(Span::styled(raw[start..end].to_string(), style));
    }
    Line::from(spans)
}

fn literal_match_ranges(raw: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let lowercase = raw.to_lowercase();
    let needle = query.to_lowercase();
    lowercase
        .match_indices(&needle)
        .filter_map(|(start, found)| {
            let end = start + found.len();
            (raw.is_char_boundary(start) && raw.is_char_boundary(end)).then_some(start..end)
        })
        .collect()
}
