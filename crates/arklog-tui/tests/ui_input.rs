use arklog::{
    render_app, AppView, Clipboard, InputMode, LogTab, OverlayMode, StreamState, TextInput,
};
use ratatui::{
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    style::Color,
    Terminal,
};

struct UnusedClipboard;

impl Clipboard for UnusedClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        Ok(String::new())
    }

    fn set_text(&mut self, _text: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn long_input_keeps_the_cursor_suffix_visible() {
    let mut input = TextInput::new(format!("{}needle", "x".repeat(90)));
    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            &mut UnusedClipboard,
        )
        .expect("select all");
    let backend = TestBackend::new(80, 20);
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
                    input_mode: InputMode::Filter,
                    overlay: OverlayMode::None,
                    input: input.view(),
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
        .chunks(80)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("needle"));
    assert!(terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .any(|cell| cell.symbol() == "n" && cell.bg == Color::Rgb(249, 226, 175)));
}
