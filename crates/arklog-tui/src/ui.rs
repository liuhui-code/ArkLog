use arklog_core::{DeviceFaultLogFetchResult, DeviceLogDevice};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};
use regex::Regex;

use crate::{input_ui::render_text_input, theme, LogTab, TextInputView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Find,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayMode {
    None,
    Devices,
}

pub struct AppView<'a> {
    pub device_id: Option<&'a str>,
    pub device_status: &'a str,
    pub devices: &'a [DeviceLogDevice],
    pub selected_device: usize,
    pub tab: LogTab,
    pub streaming: bool,
    pub connection_status: &'a str,
    pub stream_status: &'a str,
    pub fault_status: &'a str,
    pub action_error: Option<&'a str>,
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
    pub overlay: OverlayMode,
    pub input: TextInputView<'a>,
    pub fault_result: Option<&'a DeviceFaultLogFetchResult>,
    pub selected_fault: usize,
    pub fault_scroll: usize,
}

pub fn render_app(frame: &mut Frame, view: AppView<'_>) {
    frame.render_widget(Block::new().style(theme::canvas()), frame.area());
    if frame.area().width < 72 || frame.area().height < 16 {
        frame.render_widget(
            Paragraph::new("ArkLog needs at least 72 x 16 terminal cells")
                .block(theme::panel(" TERMINAL TOO SMALL ")),
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
    if view.overlay == OverlayMode::Devices {
        render_devices(frame, workspace, &view);
    } else {
        match view.tab {
            LogTab::HiLog => render_hilog(frame, workspace, &view),
            LogTab::FaultLog => render_fault_log(frame, workspace, &view),
        }
    }
    render_footer(frame, footer, &view);
}

fn render_header(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let connection_is_ready = view.connection_status == "Devices ready";
    let constraints = if view.device_id.is_none() {
        [
            Constraint::Length(56),
            Constraint::Fill(1),
            Constraint::Length(0),
        ]
    } else if !connection_is_ready {
        [
            Constraint::Length(66),
            Constraint::Fill(1),
            Constraint::Length(30),
        ]
    } else {
        [
            Constraint::Length(29),
            Constraint::Fill(1),
            Constraint::Length(30),
        ]
    };
    let [device, tabs, stream] = Layout::horizontal(constraints).areas(area);
    let device_line = match view.device_id {
        Some(id) => {
            let mut spans = vec![
                Span::styled("● ", Style::new().fg(connection_color(view.device_status))),
                Span::styled(id, Style::new().add_modifier(Modifier::BOLD)),
                Span::raw(format!("  {}", view.device_status.to_ascii_uppercase())),
            ];
            if !connection_is_ready {
                spans.push(Span::raw(format!(" · {}", view.connection_status)));
            }
            Line::from(spans)
        }
        None if view.connection_status == "No devices" => {
            Line::styled("No devices", Style::new().fg(theme::WARNING))
        }
        None => Line::styled(
            format!("No devices · {}", view.connection_status),
            Style::new().fg(theme::WARNING),
        ),
    };
    frame.render_widget(
        Paragraph::new(device_line).block(theme::panel(" DEVICE ")),
        device,
    );
    frame.render_widget(
        Tabs::new([" HiLog ", " Fault Log "])
            .select(usize::from(view.tab == LogTab::FaultLog))
            .divider("│")
            .highlight_style(
                Style::new()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .block(theme::panel(" LOG VIEWS ")),
        tabs,
    );
    let (stream_label, color) = if view.streaming {
        ("● LIVE", theme::SUCCESS)
    } else {
        ("■ STOPPED", theme::WARNING)
    };
    let stream_line = if view.device_id.is_none() {
        Line::from("")
    } else {
        Line::from(vec![
            Span::styled(
                stream_label,
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  {}", view.stream_status)),
        ])
    };
    if view.device_id.is_some() {
        frame.render_widget(
            Paragraph::new(stream_line).block(theme::panel(" STREAM ")),
            stream,
        );
    }
}

fn render_query(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let (title, value, color) = if view.overlay == OverlayMode::Devices {
        (
            " CONNECTED DEVICES · Ctrl+R refresh · ESC CLOSE ",
            view.connection_status,
            theme::INFO,
        )
    } else {
        match view.input_mode {
            InputMode::Filter => (
                " REGEX FILTER · ENTER APPLY · Ctrl+A/C/X/V/Z ",
                view.input.text,
                theme::WARNING,
            ),
            InputMode::Find => (
                " FIND IN HILOG · ENTER NEXT · Ctrl+A/C/X/V/Z ",
                view.input.text,
                theme::ACCENT,
            ),
            InputMode::Normal if view.tab == LogTab::FaultLog => (
                " FAULT LOG · Ctrl+R REFRESH ",
                "Inspect raw device diagnostics",
                theme::MUTED,
            ),
            InputMode::Normal => (
                " REGEX FILTER · Ctrl+E TO EDIT ",
                if view.filter_query.is_empty() {
                    "<all raw logs>"
                } else {
                    view.filter_query
                },
                theme::MUTED,
            ),
        }
    };
    if matches!(view.input_mode, InputMode::Filter | InputMode::Find) {
        render_text_input(frame, area, title, view.input, color);
        return;
    }
    let value = view.filter_error.unwrap_or(value);
    let color = if view.filter_error.is_some() {
        theme::ERROR
    } else {
        color
    };
    frame.render_widget(
        Paragraph::new(Span::styled(value, Style::new().fg(color))).block(theme::panel(title)),
        area,
    );
}

fn render_devices(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    if view.devices.is_empty() {
        frame.render_widget(
            Paragraph::new(view.connection_status).block(theme::panel(" CONNECTED DEVICES · 0 ")),
            area,
        );
        return;
    }
    let height = area.height.saturating_sub(2).max(1) as usize;
    let selected = view.selected_device.min(view.devices.len() - 1);
    let latest_start = view.devices.len().saturating_sub(height);
    let start = selected
        .saturating_add(1)
        .saturating_sub(height)
        .min(latest_start);
    let rows = view
        .devices
        .iter()
        .enumerate()
        .skip(start)
        .take(height)
        .map(|(index, device)| {
            let selected = index == view.selected_device;
            let status_color = connection_color(&device.status);
            Line::from(vec![
                Span::styled(
                    if selected { "▶ " } else { "  " },
                    Style::new().fg(theme::INFO),
                ),
                Span::styled(
                    format!("{:<20}", device.id),
                    Style::new()
                        .add_modifier(selected.then_some(Modifier::BOLD).unwrap_or_default()),
                ),
                Span::styled(
                    format!("{:<13}", device.status.to_ascii_uppercase()),
                    Style::new().fg(status_color),
                ),
                Span::raw(device.detail.clone()),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(rows).block(theme::panel(format!(
            " CONNECTED DEVICES · {} ",
            view.devices.len()
        ))),
        area,
    );
}

fn connection_color(status: &str) -> Color {
    match status {
        "online" => theme::SUCCESS,
        "offline" => theme::WARNING,
        _ => theme::ERROR,
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
        " SCROLL PAUSED · Ctrl+G TO LATEST "
    };
    frame.render_widget(Paragraph::new(lines).block(theme::panel(title)), area);
}

fn render_fault_log(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let Some(result) = view.fault_result else {
        frame.render_widget(
            Paragraph::new(view.fault_status).block(theme::panel(" FAULT LOG ")),
            area,
        );
        return;
    };
    if result.entries.is_empty() {
        frame.render_widget(
            Paragraph::new(result.message.as_str()).block(theme::panel(" FAULT LOG ")),
            area,
        );
        return;
    }
    let [list, inspector] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Fill(1)]).areas(area);
    let list_height = list.height.saturating_sub(2).max(1) as usize;
    let selected = view.selected_fault.min(result.entries.len() - 1);
    let list_start = selected
        .saturating_add(1)
        .saturating_sub(list_height)
        .min(result.entries.len().saturating_sub(list_height));
    let rows = result
        .entries
        .iter()
        .enumerate()
        .skip(list_start)
        .take(list_height)
        .map(|(index, entry)| {
            let style = if index == view.selected_fault {
                Style::new()
                    .fg(theme::INFO)
                    .bg(theme::SURFACE_0)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };
            Line::styled(format!("{}  {}", index + 1, entry.id), style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(rows).block(theme::panel(" ENTRIES ")), list);
    let raw = &result.entries[selected].raw;
    let inspector_height = inspector.height.saturating_sub(2).max(1) as usize;
    let visible_raw = raw
        .lines()
        .skip(view.fault_scroll)
        .take(inspector_height)
        .map(|line| Line::raw(line.to_string()))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(visible_raw).block(theme::panel(" RAW DIAGNOSTIC ")),
        inspector,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let status = view.action_error.map_or_else(
        || {
            Line::from(format!(
                " {:>7} / {:>7} VISIBLE  │  FIND {}/{}",
                view.visible_count, view.raw_count, view.find_status.0, view.find_status.1
            ))
        },
        |error| Line::styled(format!(" ERROR · {error}"), Style::new().fg(theme::ERROR)),
    );
    let shortcuts = if view.overlay == OverlayMode::Devices {
        " ←/→ select  Esc close  Ctrl+R refresh  Ctrl+S stream  Ctrl+D close  Ctrl+Q quit"
    } else if view.tab == LogTab::FaultLog {
        " ↑/↓ entry  PgUp/PgDn inspect  Ctrl+R refresh  Ctrl+D devices  Ctrl+S stream  Ctrl+Q quit"
    } else {
        " Tab view  Ctrl+D devices  Ctrl+S stream  Ctrl+R refresh  Ctrl+E regex  Ctrl+F find  Ctrl+Q quit"
    };
    frame.render_widget(
        Paragraph::new(vec![
            status,
            Line::styled(shortcuts, Style::new().fg(theme::MUTED)),
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
    let mut base = theme::log_line(raw);
    if current_find {
        base = theme::current_match_line(base);
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
            style = theme::find_match(style);
        }
        if filter_ranges.iter().any(|range| range.contains(&start)) {
            style = theme::filter_match(style);
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
