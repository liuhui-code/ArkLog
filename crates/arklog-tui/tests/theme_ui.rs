use arklog::{render_app, AppView, InputMode, LogTab, OverlayMode, TextInputView};
use ratatui::{
    backend::TestBackend,
    style::{Color, Modifier},
    Terminal,
};
use regex::Regex;

fn render_hilog(
    lines: &[String],
    filter: Option<&Regex>,
    find_query: &str,
    current_find_visible_index: Option<u64>,
) -> Terminal<TestBackend> {
    let backend = TestBackend::new(110, 28);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: Some("USB-01"),
                    device_status: "online",
                    devices: &[],
                    selected_device: 0,
                    tab: LogTab::HiLog,
                    streaming: true,
                    connection_status: "Devices ready",
                    stream_status: "Streaming",
                    fault_status: "Not loaded",
                    action_error: None,
                    raw_count: lines.len() as u64,
                    visible_count: lines.len() as u64,
                    following_latest: true,
                    filter_query: "",
                    active_filter: filter,
                    filter_error: None,
                    find_query,
                    find_status: (0, 0),
                    current_find_visible_index,
                    window_start: 0,
                    lines,
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::None,
                    input: TextInputView::at_end(""),
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
                },
            );
        })
        .expect("draw");
    terminal
}

fn cell_index(buffer: &ratatui::buffer::Buffer, text: &str) -> usize {
    let width = text.chars().count();
    buffer
        .content()
        .windows(width)
        .position(|cells| cells.iter().map(|cell| cell.symbol()).collect::<String>() == text)
        .unwrap_or_else(|| panic!("missing rendered text: {text}"))
}

#[test]
fn renders_a_cohesive_dark_canvas_with_distinct_panel_borders() {
    let lines = vec!["08-31 20:10:01.001 120 120 I ArkUI/App page shown".to_string()];
    let terminal = render_hilog(&lines, None, "", None);

    let buffer = terminal.backend().buffer();
    assert!(buffer
        .content()
        .iter()
        .all(|cell| cell.bg == Color::Rgb(30, 30, 46)));
    assert_eq!(buffer[(0, 0)].symbol(), "╭");
    assert_eq!(buffer[(0, 0)].fg, Color::Rgb(108, 112, 134));
    assert_eq!(buffer[(3, 1)].fg, Color::Rgb(205, 214, 244));
}

#[test]
fn gives_each_log_level_and_query_state_a_semantic_high_contrast_style() {
    let lines = [
        "0 F fatal",
        "0 E error",
        "0 W warning",
        "0 I info",
        "0 D debug",
        "0 V verbose",
        "0 X matched needle",
    ]
    .map(str::to_string);
    let filter = Regex::new("matched").expect("regex");
    let terminal = render_hilog(&lines, Some(&filter), "needle", Some(6));
    let buffer = terminal.backend().buffer();

    assert_eq!(buffer[(1, 7)].fg, Color::Rgb(243, 139, 168));
    assert!(buffer[(1, 7)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(1, 8)].fg, Color::Rgb(243, 139, 168));
    assert_eq!(buffer[(1, 9)].fg, Color::Rgb(249, 226, 175));
    assert_eq!(buffer[(1, 10)].fg, Color::Rgb(166, 227, 161));
    assert_eq!(buffer[(1, 11)].fg, Color::Rgb(137, 180, 250));
    assert_eq!(buffer[(1, 12)].fg, Color::Rgb(147, 153, 178));

    assert_eq!(buffer[(1, 13)].bg, Color::Rgb(49, 50, 68));
    assert_eq!(buffer[(5, 13)].fg, Color::Rgb(30, 30, 46));
    assert_eq!(buffer[(5, 13)].bg, Color::Rgb(249, 226, 175));
    assert_eq!(buffer[(13, 13)].fg, Color::Rgb(203, 166, 247));
    assert!(buffer[(13, 13)].modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn uses_one_semantic_palette_for_navigation_status_and_secondary_text() {
    let lines = vec!["0 I ready".to_string()];
    let terminal = render_hilog(&lines, None, "", None);
    let buffer = terminal.backend().buffer();

    let tab = &buffer.content()[cell_index(buffer, "HiLog")];
    assert_eq!(tab.fg, Color::Rgb(203, 166, 247));
    assert!(tab.modifier.contains(Modifier::BOLD));
    assert!(tab.modifier.contains(Modifier::UNDERLINED));

    let live = &buffer.content()[cell_index(buffer, "● LIVE")];
    assert_eq!(live.fg, Color::Rgb(166, 227, 161));
    assert!(live.modifier.contains(Modifier::BOLD));

    let query = &buffer.content()[cell_index(buffer, "<all raw logs>")];
    assert_eq!(query.fg, Color::Rgb(147, 153, 178));
    let shortcuts = &buffer.content()[cell_index(buffer, "Tab view")];
    assert_eq!(shortcuts.fg, Color::Rgb(147, 153, 178));
}
