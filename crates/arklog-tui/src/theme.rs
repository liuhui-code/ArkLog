use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType},
};

const BASE: Color = Color::Rgb(30, 30, 46);
const TEXT: Color = Color::Rgb(205, 214, 244);
const SURFACE_0: Color = Color::Rgb(49, 50, 68);
const BORDER: Color = Color::Rgb(108, 112, 134);
const MUTED: Color = Color::Rgb(147, 153, 178);
const SUBTEXT: Color = Color::Rgb(166, 173, 200);
const MAUVE: Color = Color::Rgb(203, 166, 247);
const RED: Color = Color::Rgb(243, 139, 168);
const YELLOW: Color = Color::Rgb(249, 226, 175);
const GREEN: Color = Color::Rgb(166, 227, 161);
const BLUE: Color = Color::Rgb(137, 180, 250);

pub(crate) const MIN_TERMINAL_WIDTH: u16 = 72;
pub(crate) const MIN_TERMINAL_HEIGHT: u16 = 16;
pub(crate) const TOP_CONTROLS_HEIGHT: u16 = 3;
pub(crate) const NARROW_CONTROLS_BREAKPOINT: u16 = 96;
pub(crate) const EDITING_DEVICE_WIDTH: u16 = 16;
pub(crate) const NARROW_EMPTY_DEVICE_WIDTH: u16 = 32;
pub(crate) const NARROW_DEVICE_WIDTH: u16 = 16;
pub(crate) const NARROW_STREAM_STATUS_WIDTH: u16 = 16;
pub(crate) const EMPTY_DEVICE_WIDTH: u16 = 50;
pub(crate) const STALE_DEVICE_WIDTH: u16 = 18;
pub(crate) const READY_DEVICE_WIDTH: u16 = 29;
pub(crate) const LOG_TABS_WIDTH: u16 = 24;
pub(crate) const STREAM_STATUS_WIDTH: u16 = 30;
pub(crate) const COMPACT_STREAM_STATUS_WIDTH: u16 = 18;
pub(crate) const HIDDEN_STREAM_WIDTH: u16 = 0;
pub(crate) const FAULT_LIST_PERCENT: u16 = 32;
pub(crate) const DEVICE_ID_COLUMN_WIDTH: usize = 20;
pub(crate) const DEVICE_STATUS_COLUMN_WIDTH: usize = 13;
pub(crate) const DEVICE_FIELD_GAP: &str = "  ";
pub(crate) const STATUS_SEPARATOR: &str = " · ";
pub(crate) const SELECTED_ROW_MARKER: &str = "▶ ";
pub(crate) const UNSELECTED_ROW_MARKER: &str = "  ";
pub(crate) const MIN_TERMINAL_MESSAGE: &str = "ArkLog needs at least 72 x 16 terminal cells";

const PANEL_VERTICAL_BORDER_CELLS: u16 = 2;
const MIN_PANEL_CONTENT_ROWS: u16 = 1;
const PANEL_HORIZONTAL_BORDER_CELLS: u16 = 2;
const PANEL_CONTENT_OFFSET: u16 = 1;
const INPUT_CURSOR_RESERVE: usize = 1;

#[derive(Clone, Copy)]
pub(crate) enum Tone {
    Accent,
    Info,
    Success,
    Warning,
    Error,
    Muted,
}

fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Accent => MAUVE,
        Tone::Info => BLUE,
        Tone::Success => GREEN,
        Tone::Warning => YELLOW,
        Tone::Error => RED,
        Tone::Muted => MUTED,
    }
}

pub(crate) fn canvas() -> Style {
    Style::new().fg(TEXT).bg(BASE)
}

pub(crate) fn panel<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(BORDER))
        .title(
            title
                .into()
                .style(Style::new().fg(SUBTEXT).add_modifier(Modifier::BOLD)),
        )
}

pub(crate) fn text(tone: Tone) -> Style {
    Style::new().fg(tone_color(tone))
}

pub(crate) fn emphasized_text() -> Style {
    Style::new().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub(crate) fn strong_text(tone: Tone) -> Style {
    text(tone).add_modifier(Modifier::BOLD)
}

pub(crate) fn selected_tab() -> Style {
    text(Tone::Accent).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub(crate) fn connection_status(status: &str) -> Style {
    match status {
        "online" => text(Tone::Success),
        "offline" => text(Tone::Warning),
        _ => text(Tone::Error),
    }
}

pub(crate) fn selected_row(selected: bool) -> Style {
    if selected {
        Style::new().bg(SURFACE_0)
    } else {
        Style::new()
    }
}

pub(crate) fn panel_content_height(height: u16) -> usize {
    height
        .saturating_sub(PANEL_VERTICAL_BORDER_CELLS)
        .max(MIN_PANEL_CONTENT_ROWS) as usize
}

pub(crate) fn input_content_width(area: Rect) -> usize {
    area.width.saturating_sub(PANEL_HORIZONTAL_BORDER_CELLS) as usize
}

pub(crate) fn input_cursor_limit(width: usize) -> usize {
    width.saturating_sub(INPUT_CURSOR_RESERVE)
}

pub(crate) fn input_cursor_position(area: Rect, cursor_column: usize, width: usize) -> (u16, u16) {
    (
        area.x.saturating_add(
            PANEL_CONTENT_OFFSET + cursor_column.min(input_cursor_limit(width)) as u16,
        ),
        area.y + PANEL_CONTENT_OFFSET,
    )
}

#[derive(Clone, Copy)]
pub(crate) enum LogLevel {
    Fatal,
    Error,
    Warning,
    Info,
    Debug,
    Verbose,
}

impl LogLevel {
    pub(crate) fn parse(field: &str) -> Option<Self> {
        match field {
            "F" => Some(Self::Fatal),
            "E" => Some(Self::Error),
            "W" => Some(Self::Warning),
            "I" => Some(Self::Info),
            "D" => Some(Self::Debug),
            "V" => Some(Self::Verbose),
            _ => None,
        }
    }
}

pub(crate) fn log_span(
    level: Option<LogLevel>,
    current_match_line: bool,
    find_match: bool,
    filter_match: bool,
) -> Style {
    let mut style = Style::new().fg(TEXT);
    if current_match_line {
        style = style.bg(SURFACE_0);
    }
    if let Some(level) = level {
        style = match level {
            LogLevel::Fatal => style.fg(RED).add_modifier(Modifier::BOLD),
            LogLevel::Error => style.fg(RED),
            LogLevel::Warning => style.fg(YELLOW),
            LogLevel::Info => style.fg(GREEN),
            LogLevel::Debug => style.fg(BLUE),
            LogLevel::Verbose => style.fg(MUTED),
        };
    }
    if find_match {
        style = style.fg(MAUVE).add_modifier(Modifier::UNDERLINED);
    }
    if filter_match {
        style = style.fg(BASE).bg(YELLOW).add_modifier(Modifier::BOLD);
    }
    style
}

pub(crate) fn input_selection(tone: Tone) -> Style {
    Style::new()
        .fg(BASE)
        .bg(tone_color(tone))
        .add_modifier(Modifier::BOLD)
}
