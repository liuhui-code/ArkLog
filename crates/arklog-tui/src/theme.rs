use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType},
};

pub(crate) const BASE: Color = Color::Rgb(30, 30, 46);
pub(crate) const TEXT: Color = Color::Rgb(205, 214, 244);
pub(crate) const SURFACE_0: Color = Color::Rgb(49, 50, 68);
pub(crate) const BORDER: Color = Color::Rgb(108, 112, 134);
pub(crate) const MUTED: Color = Color::Rgb(147, 153, 178);
pub(crate) const SUBTEXT: Color = Color::Rgb(166, 173, 200);
pub(crate) const MAUVE: Color = Color::Rgb(203, 166, 247);
pub(crate) const RED: Color = Color::Rgb(243, 139, 168);
pub(crate) const YELLOW: Color = Color::Rgb(249, 226, 175);
pub(crate) const GREEN: Color = Color::Rgb(166, 227, 161);
pub(crate) const BLUE: Color = Color::Rgb(137, 180, 250);
pub(crate) const ACCENT: Color = MAUVE;
pub(crate) const INFO: Color = BLUE;
pub(crate) const SUCCESS: Color = GREEN;
pub(crate) const WARNING: Color = YELLOW;
pub(crate) const ERROR: Color = RED;

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

pub(crate) fn log_line(raw: &str) -> Style {
    if raw.contains(" F ") {
        Style::new().fg(RED).add_modifier(Modifier::BOLD)
    } else if raw.contains(" E ") {
        Style::new().fg(RED)
    } else if raw.contains(" W ") {
        Style::new().fg(YELLOW)
    } else if raw.contains(" I ") {
        Style::new().fg(GREEN)
    } else if raw.contains(" D ") {
        Style::new().fg(BLUE)
    } else if raw.contains(" V ") {
        Style::new().fg(MUTED)
    } else {
        Style::new().fg(TEXT)
    }
}

pub(crate) fn current_match_line(style: Style) -> Style {
    style.bg(SURFACE_0).add_modifier(Modifier::BOLD)
}

pub(crate) fn find_match(style: Style) -> Style {
    style.fg(MAUVE).add_modifier(Modifier::UNDERLINED)
}

pub(crate) fn filter_match(style: Style) -> Style {
    style.fg(BASE).bg(YELLOW).add_modifier(Modifier::BOLD)
}

pub(crate) fn input_selection(accent: Color) -> Style {
    Style::new()
        .fg(BASE)
        .bg(accent)
        .add_modifier(Modifier::BOLD)
}
