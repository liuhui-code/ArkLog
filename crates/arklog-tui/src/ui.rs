use arklog_core::{DeviceFaultLogFetchResult, DeviceLogDevice};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};
use regex::Regex;

use crate::{
    input_ui::render_text_input,
    log_geometry::{horizontal_slice, literal_match_ranges_in, wrapped_slices, VisibleGrapheme},
    theme, LogTab, StreamAction, StreamState, TextInputView,
};

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
    pub stream_state: &'a StreamState,
    pub pending_stream_action: Option<StreamAction>,
    pub connection_status: &'a str,
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
    pub window_start_cell: u64,
    pub horizontal_offset: u64,
    pub soft_wrap: bool,
    pub lines: &'a [String],
    pub input_mode: InputMode,
    pub overlay: OverlayMode,
    pub input: TextInputView<'a>,
    pub fault_result: Option<&'a DeviceFaultLogFetchResult>,
    pub selected_fault: usize,
    pub fault_scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppLayoutGeometry {
    pub controls: Rect,
    pub workspace: Rect,
    pub log_content_height: usize,
    pub log_content_width: usize,
}

pub fn app_layout(area: Rect) -> AppLayoutGeometry {
    let [controls, workspace] = Layout::vertical([
        Constraint::Length(theme::TOP_CONTROLS_HEIGHT),
        Constraint::Fill(1),
    ])
    .areas(area);
    AppLayoutGeometry {
        controls,
        workspace,
        log_content_height: theme::panel_content_height(workspace.height),
        log_content_width: theme::panel_content_width(workspace.width),
    }
}

pub fn render_app(frame: &mut Frame, view: AppView<'_>) {
    frame.render_widget(Block::new().style(theme::canvas()), frame.area());
    if frame.area().width < theme::MIN_TERMINAL_WIDTH
        || frame.area().height < theme::MIN_TERMINAL_HEIGHT
    {
        frame.render_widget(
            Paragraph::new(theme::MIN_TERMINAL_MESSAGE).block(theme::panel(" TERMINAL TOO SMALL ")),
            frame.area(),
        );
        return;
    }
    let layout = app_layout(frame.area());
    render_header(frame, layout.controls, &view);
    if view.overlay == OverlayMode::Devices {
        render_devices(frame, layout.workspace, &view);
    } else {
        match view.tab {
            LogTab::HiLog => render_hilog(frame, layout.workspace, &view),
            LogTab::FaultLog => render_fault_log(frame, layout.workspace, &view),
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let editing_query = matches!(view.input_mode, InputMode::Filter | InputMode::Find);
    let narrow = area.width < theme::NARROW_CONTROLS_BREAKPOINT;
    let constraints = if editing_query {
        [
            Constraint::Length(theme::EDITING_DEVICE_WIDTH),
            Constraint::Length(theme::LOG_TABS_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(if view.device_id.is_some() {
                theme::NARROW_STREAM_STATUS_WIDTH
            } else {
                theme::HIDDEN_STREAM_WIDTH
            }),
        ]
    } else if view.device_id.is_none() && narrow {
        [
            Constraint::Length(theme::NARROW_EMPTY_DEVICE_WIDTH),
            Constraint::Length(theme::LOG_TABS_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(theme::HIDDEN_STREAM_WIDTH),
        ]
    } else if view.device_id.is_none() {
        [
            Constraint::Length(theme::EMPTY_DEVICE_WIDTH),
            Constraint::Length(theme::LOG_TABS_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(theme::HIDDEN_STREAM_WIDTH),
        ]
    } else if narrow {
        [
            Constraint::Length(theme::NARROW_DEVICE_WIDTH),
            Constraint::Length(theme::LOG_TABS_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(theme::NARROW_STREAM_STATUS_WIDTH),
        ]
    } else {
        [
            Constraint::Length(theme::CONNECTED_DEVICE_WIDTH),
            Constraint::Length(theme::LOG_TABS_WIDTH),
            Constraint::Fill(1),
            Constraint::Length(theme::CONNECTED_STREAM_STATUS_WIDTH),
        ]
    };
    let [device, tabs, query, stream] = Layout::horizontal(constraints).areas(area);
    let pending_hint = match view.pending_stream_action {
        Some(StreamAction::Start) => " · START QUEUED",
        Some(StreamAction::Stop) => " · STOP QUEUED",
        None => "",
    };
    let device_line = match view.device_id {
        Some(id) => Line::from(vec![
            Span::styled("● ", theme::connection_status(view.device_status)),
            Span::styled(id, theme::emphasized_text()),
            Span::raw(format!(
                "{}{}",
                theme::DEVICE_FIELD_GAP,
                view.device_status.to_ascii_uppercase()
            )),
        ]),
        None if view.connection_status == "No devices" => Line::styled(
            format!("No devices{pending_hint}"),
            theme::text(theme::Tone::Warning),
        ),
        None if view.connection_status == "Select an online device" => Line::styled(
            format!("Select device{pending_hint}"),
            theme::text(theme::Tone::Warning),
        ),
        None => Line::styled(
            format!("No devices · {}{pending_hint}", view.connection_status),
            theme::text(theme::Tone::Warning),
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
            .highlight_style(theme::selected_tab())
            .block(theme::panel(" LOG VIEWS ")),
        tabs,
    );
    render_query(frame, query, view);
    let (stream_label, tone) = match view.stream_state {
        StreamState::Starting => ("◌ STARTING", theme::Tone::Info),
        StreamState::Streaming => ("● LIVE", theme::Tone::Success),
        StreamState::Stopping => ("◌ STOPPING", theme::Tone::Warning),
        StreamState::Stopped => ("■ STOPPED", theme::Tone::Warning),
        StreamState::Error { active: true, .. } => ("! ACTIVE", theme::Tone::Error),
        StreamState::Error { active: false, .. } => ("! STOPPED", theme::Tone::Error),
    };
    let stream_detail = match (view.stream_state, view.pending_stream_action) {
        (StreamState::Stopping, Some(StreamAction::Start)) => "RESTART QUEUED",
        (StreamState::Starting, Some(StreamAction::Stop)) => "STOP QUEUED",
        _ => view.stream_state.message(),
    };
    let stream_line = if view.device_id.is_none() {
        Line::from("")
    } else {
        Line::from(vec![
            Span::styled(stream_label, theme::strong_text(tone)),
            Span::raw(format!("{}{stream_detail}", theme::DEVICE_FIELD_GAP)),
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
    let (title, value, tone) = if view.overlay == OverlayMode::Devices {
        (
            " CONNECTED DEVICES · Ctrl+R refresh · ESC CLOSE ",
            view.connection_status,
            theme::Tone::Info,
        )
    } else {
        match view.input_mode {
            InputMode::Filter => (
                " REGEX FILTER · ENTER APPLY · ↑/↓ HISTORY · Ctrl+A/C/X/V/Z ",
                view.input.text,
                theme::Tone::Warning,
            ),
            InputMode::Find => (
                " FIND IN HILOG · ENTER NEXT · Ctrl+A/C/X/V/Z ",
                view.input.text,
                theme::Tone::Accent,
            ),
            InputMode::Normal
                if view.connection_status != "Devices ready"
                    && view.connection_status != "No devices" =>
            {
                (" CONNECTION ", view.connection_status, theme::Tone::Error)
            }
            InputMode::Normal if view.tab == LogTab::FaultLog => (
                " FAULT LOG · Ctrl+R REFRESH ",
                "Inspect raw device diagnostics",
                theme::Tone::Muted,
            ),
            InputMode::Normal => (
                " REGEX FILTER · Ctrl+E TO EDIT ",
                if view.filter_query.is_empty() {
                    "<all raw logs>"
                } else {
                    view.filter_query
                },
                theme::Tone::Muted,
            ),
        }
    };
    if matches!(view.input_mode, InputMode::Filter | InputMode::Find) {
        render_text_input(frame, area, title, view.input, tone);
        return;
    }
    let error = view.action_error.or(view.filter_error);
    let value = error.unwrap_or(value);
    let tone = if error.is_some() {
        theme::Tone::Error
    } else {
        tone
    };
    frame.render_widget(
        Paragraph::new(Span::styled(value, theme::text(tone))).block(theme::panel(title)),
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
    let height = theme::panel_content_height(area.height);
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
            Line::from(vec![
                Span::styled(
                    if selected {
                        theme::SELECTED_ROW_MARKER
                    } else {
                        theme::UNSELECTED_ROW_MARKER
                    },
                    theme::text(theme::Tone::Info),
                ),
                Span::styled(
                    format!(
                        "{:<width$}",
                        device.id,
                        width = theme::DEVICE_ID_COLUMN_WIDTH
                    ),
                    theme::emphasized_text(),
                ),
                Span::styled(
                    format!(
                        "{:<width$}",
                        device.status.to_ascii_uppercase(),
                        width = theme::DEVICE_STATUS_COLUMN_WIDTH
                    ),
                    theme::connection_status(&device.status),
                ),
                Span::raw(device.detail.clone()),
            ])
            .style(theme::selected_row(selected))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(rows).block(theme::panel(format!(
            " CONNECTED DEVICES · {} · Ctrl+R refresh ",
            view.devices.len()
        ))),
        area,
    );
}

fn render_hilog(frame: &mut Frame, area: Rect, view: &AppView<'_>) {
    let content_width = theme::panel_content_width(area.width);
    let content_height = theme::panel_content_height(area.height);
    let mut lines = Vec::with_capacity(content_height);
    for (offset, raw) in view.lines.iter().enumerate() {
        let visible_index = view.window_start + offset as u64;
        let current = view.current_find_visible_index == Some(visible_index);
        if view.soft_wrap {
            let start_cell = if offset == 0 {
                view.window_start_cell
            } else {
                0
            };
            lines.extend(styled_wrapped_log_lines(
                raw,
                view.active_filter,
                view.find_query,
                current,
                start_cell,
                content_width,
                content_height.saturating_sub(lines.len()),
            ));
        } else {
            lines.push(styled_log_line(
                raw,
                view.active_filter,
                view.find_query,
                current,
                view.horizontal_offset,
                content_width,
            ));
        }
        if lines.len() == content_height {
            break;
        }
    }
    let mode = if view.following_latest {
        "FOLLOWING LATEST"
    } else {
        "SCROLL PAUSED · Ctrl+G TO LATEST"
    };
    let wrap = if view.soft_wrap { " · WRAP ON" } else { "" };
    let title = format!(
        " {mode}{wrap} · {} / {} VISIBLE · FIND {}/{} ",
        view.visible_count, view.raw_count, view.find_status.0, view.find_status.1
    );
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
    let [list, inspector] = Layout::horizontal([
        Constraint::Percentage(theme::FAULT_LIST_PERCENT),
        Constraint::Fill(1),
    ])
    .areas(area);
    let list_height = theme::panel_content_height(list.height);
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
            Line::styled(
                format!("{}{}{}", index + 1, theme::DEVICE_FIELD_GAP, entry.id),
                theme::selected_row(index == view.selected_fault),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(rows).block(theme::panel(" ENTRIES ")), list);
    let raw = &result.entries[selected].raw;
    let inspector_height = theme::panel_content_height(inspector.height);
    let visible_raw = raw
        .lines()
        .skip(view.fault_scroll)
        .take(inspector_height)
        .map(|line| Line::raw(line.to_string()))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(visible_raw).block(theme::panel(" RAW DIAGNOSTIC · PgUp/PgDn inspect ")),
        inspector,
    );
}

fn styled_log_line(
    raw: &str,
    filter: Option<&Regex>,
    find_query: &str,
    current_find: bool,
    horizontal_offset: u64,
    content_width: usize,
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
    ));
    if visible.hidden_right {
        spans.push(Span::styled(
            theme::RIGHT_OVERFLOW_MARKER,
            theme::overflow_marker(current_find),
        ));
    }
    Line::from(spans)
}

fn styled_wrapped_log_lines(
    raw: &str,
    filter: Option<&Regex>,
    find_query: &str,
    current_find: bool,
    start_cell: u64,
    content_width: usize,
    limit: usize,
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
