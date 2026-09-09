use arklog::{render_app, AppView, InputMode, LogTab, OverlayMode, StreamState, TextInputView};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn minimum_width_hilog_view_maximizes_log_workspace_without_a_footer() {
    let lines = vec!["0 I live log".to_string()];
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
    assert!(rendered.contains("live log"));
    assert!(rendered.contains("HiLog"));
    assert!(rendered.contains("Fault Log"));
    assert!(rendered.contains("REGEX FILTER"));
    assert!(rendered.contains("● LIVE"));
    assert!(!rendered.contains("VISIBLE  │  FIND"));
    assert_eq!(terminal.backend().buffer()[(0, 15)].symbol(), "╰");
    assert_eq!(terminal.backend().buffer()[(71, 15)].symbol(), "╯");
}
