use arklog::{render_app, AppView, InputMode, LogTab, OverlayMode, StreamState, TextInputView};
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
                    stream_state: &StreamState::Streaming,
                    pending_stream_action: None,
                    connection_status: "Devices ready",
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
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
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
fn colors_only_the_level_field_and_keeps_every_other_log_field_neutral() {
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

    for (line, expected_level_color) in [
        ("0 F fatal", Color::Rgb(243, 139, 168)),
        ("0 E error", Color::Rgb(243, 139, 168)),
        ("0 W warning", Color::Rgb(249, 226, 175)),
        ("0 I info", Color::Rgb(166, 227, 161)),
        ("0 D debug", Color::Rgb(137, 180, 250)),
        ("0 V verbose", Color::Rgb(147, 153, 178)),
    ] {
        let start = cell_index(buffer, line);
        assert_eq!(buffer.content()[start].fg, Color::Rgb(205, 214, 244));
        assert_eq!(buffer.content()[start + 2].fg, expected_level_color);
        assert_eq!(buffer.content()[start + 4].fg, Color::Rgb(205, 214, 244));
    }
    let fatal = cell_index(buffer, "0 F fatal");
    assert!(!buffer.content()[fatal].modifier.contains(Modifier::BOLD));
    assert!(buffer.content()[fatal + 2]
        .modifier
        .contains(Modifier::BOLD));
}

#[test]
fn keeps_an_optional_line_number_neutral_while_coloring_the_hilog_level() {
    let line = "42 08-31 20:10:01.001 120 120 I ArkUI/App page shown";
    let lines = vec![line.to_string()];
    let terminal = render_hilog(&lines, None, "", None);
    let buffer = terminal.backend().buffer();

    let start = cell_index(buffer, line);
    let level = cell_index(buffer, " I ArkUI") + 1;
    let tag = cell_index(buffer, "ArkUI/App");
    assert_eq!(buffer.content()[start].fg, Color::Rgb(205, 214, 244));
    assert_eq!(buffer.content()[level].fg, Color::Rgb(166, 227, 161));
    assert_eq!(buffer.content()[tag].fg, Color::Rgb(205, 214, 244));
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
}

#[test]
fn composes_low_contrast_current_row_and_highlights_only_actual_matches() {
    let line = "08-31 20:10:01.001 120 120 I ArkUI/App needle matched";
    let lines = vec![line.to_string()];
    let filter = Regex::new("matched").expect("regex");
    let terminal = render_hilog(&lines, Some(&filter), "needle", Some(0));
    let buffer = terminal.backend().buffer();

    let start = cell_index(buffer, line);
    let level = cell_index(buffer, " I ArkUI") + 1;
    let tag = cell_index(buffer, "ArkUI/App");
    let find = cell_index(buffer, "needle");
    let filter = cell_index(buffer, "matched");

    assert_eq!(buffer.content()[start].fg, Color::Rgb(205, 214, 244));
    assert_eq!(buffer.content()[start].bg, Color::Rgb(49, 50, 68));
    assert!(!buffer.content()[start].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer.content()[level].fg, Color::Rgb(166, 227, 161));
    assert_eq!(buffer.content()[tag].fg, Color::Rgb(205, 214, 244));
    assert_eq!(buffer.content()[tag].bg, Color::Rgb(49, 50, 68));
    assert_eq!(buffer.content()[find].fg, Color::Rgb(203, 166, 247));
    assert!(buffer.content()[find]
        .modifier
        .contains(Modifier::UNDERLINED));
    assert_eq!(buffer.content()[filter].fg, Color::Rgb(30, 30, 46));
    assert_eq!(buffer.content()[filter].bg, Color::Rgb(249, 226, 175));
    assert_eq!(buffer.content()[filter - 1].fg, Color::Rgb(205, 214, 244));
    assert_eq!(buffer.content()[filter - 1].bg, Color::Rgb(49, 50, 68));
}

#[test]
fn presentation_modules_consume_semantic_tokens_instead_of_owning_design_values() {
    for (name, source) in [
        ("ui.rs", include_str!("../src/ui.rs")),
        ("input_ui.rs", include_str!("../src/input_ui.rs")),
        ("log_geometry.rs", include_str!("../src/log_geometry.rs")),
    ] {
        assert!(!source.contains("Color::"), "{name} owns a raw color");
        assert!(
            !source.contains("Style::new"),
            "{name} constructs a visual style"
        );
        assert!(
            !source.contains("Modifier::"),
            "{name} owns a visual modifier"
        );
        assert!(
            !source.contains("saturating_sub(2)"),
            "{name} owns panel inset"
        );
        assert!(
            !source.contains("max(1)"),
            "{name} owns minimum content height"
        );
        assert!(
            !source.contains("{:<20}"),
            "{name} owns device column width"
        );
        assert!(
            !source.contains("{:<13}"),
            "{name} owns status column width"
        );
        for line in source.lines() {
            for constructor in ["Constraint::Length(", "Constraint::Percentage("] {
                if let Some(value) = line.split(constructor).nth(1) {
                    assert!(
                        !value.starts_with(|character: char| character.is_ascii_digit()),
                        "{name} owns an unnamed layout value: {line}"
                    );
                }
            }
        }
    }
}
