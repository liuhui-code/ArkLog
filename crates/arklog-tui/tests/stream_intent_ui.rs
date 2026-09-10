use arklog::{
    render_app, AppView, InputMode, LogTab, OverlayMode, StreamAction, StreamState, TextInput,
};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn stopping_stream_shows_when_a_restart_is_queued() {
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
                    pending_stream_action: Some(StreamAction::Start),
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
    assert!(content.contains("RESTART QUEUED"), "{content}");
    assert!(!content.contains("■ STOPPED"), "{content}");
}

#[test]
fn initial_refresh_only_shows_start_queued_until_ctrl_s_cancels_it() {
    let queued = render_no_device_header(Some(StreamAction::Start));
    let cancelled = render_no_device_header(None);

    assert!(queued.contains("START QUEUED"), "{queued}");
    assert!(!cancelled.contains("START QUEUED"), "{cancelled}");
}

#[test]
fn connection_refresh_does_not_reflow_navigation_slots() {
    let ready = render_connected_header("Devices ready");
    let refreshing = render_connected_header("Refreshing devices");

    let ready_tabs = ready
        .lines()
        .next()
        .and_then(|line| line.find("LOG VIEWS"))
        .expect("ready log tabs");
    let refreshing_tabs = refreshing
        .lines()
        .next()
        .and_then(|line| line.find("LOG VIEWS"))
        .expect("refreshing log tabs");
    assert_eq!(ready_tabs, refreshing_tabs);
}

fn render_connected_header(connection_status: &str) -> String {
    let backend = TestBackend::new(110, 20);
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
                    stream_state: &StreamState::Streaming,
                    pending_stream_action: None,
                    connection_status,
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
    terminal.backend().to_string()
}

fn render_no_device_header(pending_stream_action: Option<StreamAction>) -> String {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let input = TextInput::new("");
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
                    pending_stream_action,
                    connection_status: "Refreshing devices",
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
    terminal.backend().to_string()
}
