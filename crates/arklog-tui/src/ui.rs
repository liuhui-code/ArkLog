use arklog_core::{DeviceFaultLogFetchResult, DeviceLogDevice};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};
use regex::Regex;

use crate::{
    input_ui::render_text_input, theme, LogSelection, LogTab, StreamAction, StreamState,
    TextInputView,
};

mod log_lines;
use log_lines::{styled_log_line, styled_wrapped_log_lines};

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
    pub log_selection: Option<&'a LogSelection>,
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
    pub log_content: Rect,
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
        log_content: if area.width < theme::MIN_TERMINAL_WIDTH
            || area.height < theme::MIN_TERMINAL_HEIGHT
        {
            Rect::default()
        } else {
            theme::panel_content_area(workspace)
        },
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
    let device_label = view.device_id.map(|id| {
        view.devices
            .iter()
            .find(|device| device.id == id)
            .map_or(id, |device| device.label.as_str())
    });
    let device_line = match device_label {
        Some(label) => Line::from(vec![
            Span::styled("● ", theme::connection_status(view.device_status)),
            Span::styled(label, theme::emphasized_text()),
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
            .block(theme::panel(" LOG VIEWS · F1/F2 ")),
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
                " REGEX FILTER · TAB COMPLETE · ENTER APPLY · ↑/↓ HISTORY ",
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
                    && view.connection_status != "No devices"
                    && view.connection_status != "Refreshing devices" =>
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
                        device.label,
                        width = theme::DEVICE_LABEL_COLUMN_WIDTH
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
        let selected = view.log_selection.map_or(0..0, |selection| {
            selection.range_for_record(visible_index, raw.len())
        });
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
                &selected,
            ));
        } else {
            lines.push(styled_log_line(
                raw,
                view.active_filter,
                view.find_query,
                current,
                view.horizontal_offset,
                content_width,
                &selected,
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
