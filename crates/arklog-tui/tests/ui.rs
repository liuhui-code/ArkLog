use arklog::{render_app, AppView, InputMode, LogTab, OverlayMode, StreamState, TextInputView};
use arklog_core::{
    DeviceFaultLogFetchResult, DeviceFaultLogRawEntry, DeviceFaultLogStatus, DeviceLogDevice,
};
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
                    devices: &[],
                    selected_device: 0,
                    tab: LogTab::HiLog,
                    stream_state: &StreamState::Stopped,
                    pending_stream_action: None,
                    connection_status: "No devices",
                    fault_status: "Not loaded",
                    action_error: None,
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
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &lines,
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
    assert!(rendered.contains("1 / 1 VISIBLE"));
    assert!(rendered.contains("FIND 0/0"));
    let buffer = terminal.backend().buffer();
    let log_start = find_cell_sequence(buffer.content(), "page shown");
    assert_eq!(
        log_start / 110,
        4,
        "HiLog should begin below one 3-row top bar"
    );
    assert_eq!(buffer[(0, 27)].symbol(), "╰");
    assert_eq!(buffer[(109, 27)].symbol(), "╯");
    assert!(!rendered.contains("VISIBLE  │  FIND"));
    assert!(!rendered.to_ascii_lowercase().contains("hilog output"));
    assert!(!rendered.to_ascii_lowercase().contains("unavailable"));
}

#[test]
fn renders_the_real_connection_failure_inside_the_no_devices_status() {
    let backend = TestBackend::new(110, 28);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: None,
                    device_status: "No devices",
                    devices: &[],
                    selected_device: 0,
                    tab: LogTab::HiLog,
                    stream_state: &StreamState::Stopped,
                    pending_stream_action: None,
                    connection_status: "Connect server failed: daemon unavailable",
                    fault_status: "Not loaded",
                    action_error: None,
                    raw_count: 0,
                    visible_count: 0,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &[],
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

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("No devices"));
    assert!(rendered.contains("Connect server failed"));

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
                    connection_status: "Connect server failed: stale snapshot",
                    fault_status: "Not loaded",
                    action_error: None,
                    raw_count: 0,
                    visible_count: 0,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &[],
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::None,
                    input: TextInputView::at_end(""),
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
                },
            );
        })
        .expect("draw stale device");
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("stale snapshot"));
}

#[test]
fn renders_all_connected_devices_in_the_device_list_view() {
    let devices = vec![
        DeviceLogDevice {
            id: "USB-01".to_string(),
            label: "TAS-AL00 USB-01".to_string(),
            status: "online".to_string(),
            detail: "USB-01 Connected product:alpha".to_string(),
        },
        DeviceLogDevice {
            id: "USB-02".to_string(),
            label: "NOH-AN00 USB-02".to_string(),
            status: "offline".to_string(),
            detail: "USB-02 Offline product:beta".to_string(),
        },
    ];
    let backend = TestBackend::new(110, 28);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: Some("USB-02"),
                    device_status: "offline",
                    devices: &devices,
                    selected_device: 1,
                    tab: LogTab::HiLog,
                    stream_state: &StreamState::Stopped,
                    pending_stream_action: None,
                    connection_status: "Devices ready",
                    fault_status: "Not loaded",
                    action_error: None,
                    raw_count: 0,
                    visible_count: 0,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &[],
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::Devices,
                    input: TextInputView::at_end(""),
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
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
    let connection_marker = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "●")
        .expect("connection marker");
    assert_eq!(connection_marker.fg, Color::Rgb(249, 226, 175));
    assert!(rendered.contains("CONNECTED DEVICES"));
    assert!(rendered.contains("TAS-AL00 USB-01"));
    assert!(rendered.contains("product:alpha"));
    assert_eq!(
        rendered.matches("NOH-AN00").count(),
        2,
        "the selected phone code should lead both the compact header and full list label"
    );
    assert!(rendered.contains("product:beta"));
    assert!(rendered.contains("Ctrl+R refresh"));
}

#[test]
fn device_list_windows_a_large_collection_around_the_selection() {
    let devices = (0..30)
        .map(|index| DeviceLogDevice {
            id: format!("USB-{index:02}"),
            label: format!("Phone {index}"),
            status: "online".to_string(),
            detail: format!("USB-{index:02} Connected"),
        })
        .collect::<Vec<_>>();
    let backend = TestBackend::new(110, 16);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| {
            render_app(
                frame,
                AppView {
                    device_id: Some("USB-29"),
                    device_status: "online",
                    devices: &devices,
                    selected_device: 29,
                    tab: LogTab::HiLog,
                    stream_state: &StreamState::Stopped,
                    pending_stream_action: None,
                    connection_status: "Devices ready",
                    fault_status: "Not loaded",
                    action_error: None,
                    raw_count: 0,
                    visible_count: 0,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &[],
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::Devices,
                    input: TextInputView::at_end(""),
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
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
    assert!(rendered.contains("USB-29"));
    assert!(rendered.contains("USB-29 Connected"));
    assert!(!rendered.contains("USB-00"));
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
                    devices: &[],
                    selected_device: 0,
                    tab: LogTab::HiLog,
                    stream_state: &StreamState::Streaming,
                    pending_stream_action: None,
                    connection_status: "Devices ready",
                    fault_status: "Not loaded",
                    action_error: None,
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
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    let first_match = find_cell_sequence(buffer.content(), "task-12");
    let second_match = find_cell_sequence(buffer.content(), "task-345");
    for offset in first_match..first_match + "task-12".len() {
        assert_eq!(buffer.content()[offset].bg, Color::Rgb(249, 226, 175));
    }
    for offset in second_match..second_match + "task-345".len() {
        assert_eq!(buffer.content()[offset].bg, Color::Rgb(249, 226, 175));
    }
}

#[test]
fn highlights_all_find_matches_and_distinguishes_the_current_match_line() {
    let lines = vec!["Needle first".to_string(), "NEEDLE second".to_string()];
    let backend = TestBackend::new(100, 20);
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
                    raw_count: 2,
                    visible_count: 2,
                    following_latest: false,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "nEeDlE",
                    find_status: (2, 2),
                    current_find_visible_index: Some(1),
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    let first = find_cell_sequence(buffer.content(), "Needle first");
    let second = find_cell_sequence(buffer.content(), "NEEDLE second");
    assert!(buffer.content()[first]
        .modifier
        .contains(ratatui::style::Modifier::UNDERLINED));
    assert_eq!(buffer.content()[second].bg, Color::Rgb(49, 50, 68));
}

#[test]
fn windows_large_fault_lists_and_raw_diagnostics_to_the_visible_area() {
    let entries = (0..30)
        .map(|index| DeviceFaultLogRawEntry {
            id: format!("fault-{index:02}"),
            raw: (0..100)
                .map(|line| format!("fault-{index:02}-line-{line:03}"))
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect();
    let result = DeviceFaultLogFetchResult {
        device_id: "USB-01".to_string(),
        entries,
        command: "hdc faultloggerd".to_string(),
        stderr: String::new(),
        status: DeviceFaultLogStatus::Ready,
        message: "ok".to_string(),
    };
    let backend = TestBackend::new(100, 18);
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
                    tab: LogTab::FaultLog,
                    stream_state: &StreamState::Streaming,
                    pending_stream_action: None,
                    connection_status: "Devices ready",
                    fault_status: "Fault logs ready",
                    action_error: None,
                    raw_count: 0,
                    visible_count: 0,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &[],
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::None,
                    input: TextInputView::at_end(""),
                    fault_result: Some(&result),
                    selected_fault: 29,
                    fault_scroll: 90,
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
    assert!(rendered.contains("fault-29"));
    assert!(!rendered.contains("fault-00"));
    assert!(rendered.contains("fault-29-line-090"));
    assert!(!rendered.contains("fault-29-line-000"));
    assert!(rendered.contains("PgUp/PgDn inspect"));
    assert!(!rendered.contains("Ctrl+F find"));
}

#[test]
fn horizontal_slice_marks_only_content_that_is_actually_hidden() {
    let lines = vec!["abcdefghijklmnopqrstuvwxyz".repeat(4), "short".to_string()];
    let backend = TestBackend::new(72, 16);
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
                    raw_count: 2,
                    visible_count: 2,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 20,
                    soft_wrap: false,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 4)].symbol(), "‹");
    assert_eq!(buffer[(2, 4)].symbol(), "u");
    assert_eq!(buffer[(70, 4)].symbol(), "›");
    assert_eq!(buffer[(1, 5)].symbol(), "‹");
    assert_ne!(buffer[(70, 5)].symbol(), "›");
}

#[test]
fn short_and_exact_width_lines_have_no_overflow_markers_at_column_zero() {
    let lines = vec!["short".to_string(), "x".repeat(70)];
    let backend = TestBackend::new(72, 16);
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
                    raw_count: 2,
                    visible_count: 2,
                    following_latest: true,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    for y in [4, 5] {
        assert_ne!(buffer[(1, y)].symbol(), "‹");
        assert_ne!(buffer[(70, y)].symbol(), "›");
    }
}

#[test]
fn huge_offsets_crop_unicode_graphemes_and_keep_find_highlight_on_source_text() {
    let raw = format!("{}中e\u{301}👩‍💻\tNEEDLE", "x".repeat(65_540));
    let lines = vec![raw.clone()];
    let backend = TestBackend::new(72, 16);
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
                    raw_count: 1,
                    visible_count: 1,
                    following_latest: false,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "needle",
                    find_status: (1, 1),
                    current_find_visible_index: Some(0),
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 65_540,
                    soft_wrap: false,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 4)].symbol(), "‹");
    assert_eq!(buffer[(2, 4)].symbol(), "中");
    let match_start = find_cell_sequence(buffer.content(), "NEEDLE");
    assert!(buffer.content()[match_start]
        .modifier
        .contains(ratatui::style::Modifier::UNDERLINED));
    assert_eq!(
        lines[0], raw,
        "rendering must not rewrite the stored record"
    );
}

#[test]
fn soft_wrap_renders_continuations_without_changing_record_count_or_source() {
    let raw = format!("{}TAIL", "a".repeat(70));
    let lines = vec![raw.clone()];
    let backend = TestBackend::new(72, 16);
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
                    raw_count: 1,
                    visible_count: 1,
                    following_latest: false,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "",
                    find_status: (0, 0),
                    current_find_visible_index: None,
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: true,
                    lines: &lines,
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

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 4)].symbol(), "a");
    assert_eq!(buffer[(1, 5)].symbol(), "↪");
    assert_eq!(buffer[(2, 5)].symbol(), "T");
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("1 / 1 VISIBLE"));
    assert!(rendered.contains("WRAP ON"));
    assert_eq!(lines[0], raw);
}

#[test]
fn a_match_inside_a_combining_grapheme_highlights_the_intact_terminal_cell() {
    let lines = vec!["e\u{301} tail".to_string()];
    let backend = TestBackend::new(72, 16);
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
                    raw_count: 1,
                    visible_count: 1,
                    following_latest: false,
                    filter_query: "",
                    active_filter: None,
                    filter_error: None,
                    find_query: "\u{301}",
                    find_status: (1, 1),
                    current_find_visible_index: Some(0),
                    window_start: 0,
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: true,
                    lines: &lines,
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

    assert_eq!(terminal.backend().buffer()[(1, 4)].symbol(), "e\u{301}");
    assert!(terminal.backend().buffer()[(1, 4)]
        .modifier
        .contains(ratatui::style::Modifier::UNDERLINED));
}

fn find_cell_sequence(cells: &[ratatui::buffer::Cell], needle: &str) -> usize {
    cells
        .windows(needle.len())
        .position(|window| window.iter().map(|cell| cell.symbol()).collect::<String>() == needle)
        .expect("cell sequence")
}
