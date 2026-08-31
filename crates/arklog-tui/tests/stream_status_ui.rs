use arklog::{render_app, AppView, InputMode, LogTab, OverlayMode, StreamState, TextInput};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn pending_stop_is_shown_as_stopping_instead_of_stopped() {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let input = TextInput::new("");

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
                    stream_state: &StreamState::Stopping,
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
                    lines: &[],
                    input_mode: InputMode::Normal,
                    overlay: OverlayMode::None,
                    input: input.view(),
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
                },
            );
        })
        .expect("draw");

    let content = terminal.backend().to_string();
    assert!(content.contains("STOPPING"), "{content}");
    assert!(!content.contains("■ STOPPED"), "{content}");
}
