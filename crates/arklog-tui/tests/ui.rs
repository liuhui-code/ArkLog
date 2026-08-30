use arklog::{render_app, AppView, InputMode, LogTab};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use regex::Regex;

#[test]
fn renders_compact_shared_workspace_without_duplicate_device_or_hilog_status() {
    let lines = vec!["08-30 20:10:01.001 120 120 I ArkUI/App page shown".to_string()];
    let backend = TestBackend::new(110, 28);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: None,
                    device_status: "No devices",
                    tab: LogTab::HiLog,
                    streaming: false,
                    runtime_status: "No devices",
                    raw_count: 1,
                    visible_count: 1,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    lines: &lines,
                    input_mode: InputMode::Normal,
                    input_draft: "",
                    fault_result: None,
                    selected_fault: 0,
                },
            );
        })
        .expect("draw");

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert_eq!(rendered.matches("No devices").count(), 1);
    assert!(rendered.contains("HiLog"));
    assert!(rendered.contains("Fault Log"));
    assert!(rendered.contains("REGEX FILTER"));
    assert!(rendered.contains("page shown"));
    assert!(rendered.contains("FOLLOWING LATEST"));
    assert!(!rendered.to_ascii_lowercase().contains("hilog output"));
    assert!(!rendered.to_ascii_lowercase().contains("unavailable"));
}

#[test]
fn highlights_every_non_empty_regex_match_without_changing_the_raw_line() {
    let lines = vec!["before task-12 after task-345".to_string()];
    let filter = Regex::new(r"task-\d+").expect("regex");
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: Some("USB-01"),
                    device_status: "online",
                    tab: LogTab::HiLog,
                    streaming: true,
                    runtime_status: "Streaming",
                    raw_count: 1,
                    visible_count: 1,
                    following_latest: true,
                    filter_query: r"task-\d+",
                    active_filter: Some(&filter),
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    lines: &lines,
                    input_mode: InputMode::Normal,
                    input_draft: "",
                    fault_result: None,
                    selected_fault: 0,
                },
            );
        })
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let first_match = find_cell_sequence(buffer.content(), "task-12");
    let second_match = find_cell_sequence(buffer.content(), "task-345");
    for offset in first_match..first_match + "task-12".len() {
        assert_eq!(buffer.content()[offset].bg, Color::Yellow);
    }
    for offset in second_match..second_match + "task-345".len() {
        assert_eq!(buffer.content()[offset].bg, Color::Yellow);
    }
}

#[test]
fn highlights_all_find_matches_and_distinguishes_the_current_match_line() {
    let lines = vec!["needle first".to_string(), "needle second".to_string()];
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: Some("USB-01"),
                    device_status: "online",
                    tab: LogTab::HiLog,
                    streaming: true,
                    runtime_status: "Streaming",
                    raw_count: 2,
                    visible_count: 2,
                    following_latest: false,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "needle",
                    find_status: (2, 2),
                    current_find_visible_index: Some(1),
                    window_start: 0,
                    lines: &lines,
                    input_mode: InputMode::Normal,
                    input_draft: "",
                    fault_result: None,
                    selected_fault: 0,
                },
            );
        })
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let first = find_cell_sequence(buffer.content(), "needle first");
    let second = find_cell_sequence(buffer.content(), "needle second");
    assert!(buffer.content()[first]
        .modifier
        .contains(ratatui::style::Modifier::UNDERLINED));
    assert_eq!(buffer.content()[second].bg, Color::DarkGray);
}

fn find_cell_sequence(cells: &[ratatui::buffer::Cell], needle: &str) -> usize {
    cells
        .windows(needle.len())
        .position(|window| window.iter().map(|cell| cell.symbol()).collect::<String>() == needle)
        .expect("cell sequence")
}
