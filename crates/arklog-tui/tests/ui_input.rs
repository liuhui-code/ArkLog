use arklog::{
    render_app, AppView, Clipboard, InputMode, LogTab, OverlayMode, StreamState, TextInput,
    TextInputView,
};
use ratatui::{
    backend::TestBackend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    style::Color,
    Terminal,
};

struct UnusedClipboard;

#[test]
fn ghost_text_is_muted_and_keeps_the_cursor_at_the_real_input_end() {
    use ratatui::backend::Backend;
    let input = TextInput::new("err");
    let mut view = input.view();
    view.suggestion = Some("or|timeout");
    let mut terminal = render_input(view, 110);
    let cells = terminal.backend().buffer().content();
    let start = cells
        .windows(13)
        .position(|cells| {
            cells.iter().map(|cell| cell.symbol()).collect::<String>() == "error|timeout"
        })
        .unwrap();
    assert_eq!(cells[start].fg, Color::Rgb(249, 226, 175));
    assert_eq!(cells[start + 3].fg, Color::Rgb(147, 153, 178));
    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
    assert_eq!(
        (cursor.x, cursor.y),
        (((start + 3) % 110) as u16, (start / 110) as u16)
    );
    assert_eq!(input.text(), "err");
}

#[test]
fn long_unicode_suggestions_are_clipped_without_covering_the_border_or_cursor() {
    use ratatui::backend::Backend;
    let suffix = "中e\u{301}👩‍💻".repeat(100);
    for width in [72, 80, 110] {
        let input = TextInput::new("err");
        let mut baseline = render_input(input.view(), width);
        let mut preview = input.view();
        preview.suggestion = Some(&suffix);
        let mut hinted = render_input(preview, width);
        assert_eq!(
            hinted.backend_mut().get_cursor_position().unwrap(),
            baseline.backend_mut().get_cursor_position().unwrap()
        );
        assert_eq!(hinted.backend().buffer()[(width - 1, 1)].symbol(), "│");
        assert_eq!(input.text(), "err");
    }
}

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
    let terminal = render_input(input.view(), 80);

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

fn render_input(input: TextInputView<'_>, width: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, 20);
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
                    window_start_cell: 0,
                    horizontal_offset: 0,
                    soft_wrap: false,
                    log_selection: None,
                    lines: &[],
                    input_mode: InputMode::Filter,
                    overlay: OverlayMode::None,
                    input,
                    fault_result: None,
                    selected_fault: 0,
                    fault_scroll: 0,
                },
            );
        })
        .expect("draw");

    terminal
}
