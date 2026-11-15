use arklog::{ArkLogState, Clipboard};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};

#[derive(Default)]
struct MemoryClipboard(String);

impl Clipboard for MemoryClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        Ok(self.0.clone())
    }
    fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.0 = text.into();
        Ok(())
    }
}

#[test]
fn shift_arrows_select_unicode_log_text_and_ctrl_c_copies_without_screen_markers() {
    let mut state = ArkLogState::new().unwrap();
    state
        .append_lines(["中e\u{301}👩‍💻\tfirst", "second"])
        .unwrap();
    state.set_viewport_size(20, 5);
    let mut clipboard = MemoryClipboard::default();
    for _ in 0..3 {
        assert!(state
            .handle_log_key(
                KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
                &mut clipboard
            )
            .unwrap());
    }
    state.append_lines(["new log"]).unwrap();
    assert!(!state.is_following_latest());
    assert_eq!(state.display_window().unwrap().start_record, 0);
    assert!(state
        .handle_log_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut clipboard
        )
        .unwrap());
    assert_eq!(clipboard.0, "中e\u{301}👩‍💻");
}

#[test]
fn keyboard_selection_crosses_records_and_escape_clears_it() {
    let mut state = ArkLogState::new().unwrap();
    state.append_lines(["one", "two"]).unwrap();
    state.set_viewport_size(20, 5);
    let mut clipboard = MemoryClipboard::default();
    for code in [KeyCode::End, KeyCode::Right, KeyCode::Right] {
        assert!(state
            .handle_log_key(KeyEvent::new(code, KeyModifiers::SHIFT), &mut clipboard)
            .unwrap());
    }
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("one\nt")
    );
    for code in [KeyCode::Left, KeyCode::Home, KeyCode::Up] {
        state
            .handle_log_key(KeyEvent::new(code, KeyModifiers::SHIFT), &mut clipboard)
            .unwrap();
    }
    assert_eq!(state.selected_log_text().unwrap(), None);
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .unwrap();
    assert_eq!(state.selected_log_text().unwrap().as_deref(), Some("one\n"));
    assert!(state
        .handle_log_key(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &mut clipboard
        )
        .unwrap());
    assert!(state.log_selection().is_none());
}

#[test]
fn select_all_copies_only_the_current_filtered_snapshot() {
    let mut state = ArkLogState::new().unwrap();
    state
        .append_lines(["keep first", "hidden", "keep last"])
        .unwrap();
    state.apply_filter("keep").unwrap();
    state.set_viewport_size(20, 5);
    let mut clipboard = MemoryClipboard::default();
    assert!(state
        .handle_log_key(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            &mut clipboard
        )
        .unwrap());
    state.append_lines(["keep appended"]).unwrap();
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .unwrap();
    assert_eq!(clipboard.0, "keep first\nkeep last");
    state.apply_filter("hidden").unwrap();
    assert!(
        state.log_selection().is_none(),
        "filter changes invalidate record ordinals"
    );
}

#[test]
fn oversized_copy_reports_an_error_and_preserves_the_clipboard() {
    let mut state = ArkLogState::new().unwrap();
    state
        .append_lines(["x".repeat(600_000), "y".repeat(600_000)])
        .unwrap();
    let mut clipboard = MemoryClipboard("previous clipboard".into());
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .unwrap();
    let error = state
        .handle_log_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .unwrap_err();
    assert!(error.contains("1 MiB"));
    assert_eq!(clipboard.0, "previous clipboard");
    state.clear().unwrap();
    assert!(state.log_selection().is_none());
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn reverse_mouse_drag_selects_raw_text_across_rows_and_survives_append() {
    let mut state = ArkLogState::new().unwrap();
    state.append_lines(["first", "second"]).unwrap();
    state.set_viewport_size(20, 5);
    let area = Rect::new(1, 4, 20, 5);
    for event in [
        mouse(MouseEventKind::Down(MouseButton::Left), 4, 5),
        mouse(MouseEventKind::Drag(MouseButton::Left), 3, 4),
        mouse(MouseEventKind::Up(MouseButton::Left), 3, 4),
    ] {
        assert!(state.handle_log_mouse(event, area).unwrap());
    }
    state.append_lines(["third"]).unwrap();
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("rst\nsec")
    );
    assert!(!state.is_following_latest());
    assert!(!state
        .handle_log_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), area)
        .unwrap());
}

#[test]
fn mouse_selection_maps_horizontal_and_wrapped_unicode_cells_to_source_bytes() {
    let mut state = ArkLogState::new().unwrap();
    let raw = "01234567中e\u{301}👩‍💻\tTAIL";
    state.append_lines([raw]).unwrap();
    state.set_viewport_size(12, 4);
    state.scroll_horizontal_right().unwrap();
    assert_eq!(state.horizontal_offset(), 8);
    let area = Rect::new(1, 4, 12, 4);
    state
        .handle_log_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 4), area)
        .unwrap();
    state
        .handle_log_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 7, 4), area)
        .unwrap();
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("中e\u{301}👩‍💻")
    );

    state.clear().unwrap();
    state.append_lines(["ab中e\u{301}👩‍💻\tTAIL"]).unwrap();
    state.set_viewport_size(8, 4);
    state.toggle_soft_wrap();
    let area = Rect::new(1, 4, 8, 4);
    state
        .handle_log_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 3, 4), area)
        .unwrap();
    state
        .handle_log_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 4, 5), area)
        .unwrap();
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("中e\u{301}👩‍💻\tTA")
    );
}

#[test]
fn extending_selection_reveals_the_caret_in_horizontal_and_wrapped_views() {
    let mut state = ArkLogState::new().unwrap();
    state
        .append_lines(["012345678901234567890123456789"])
        .unwrap();
    state.set_viewport_size(8, 2);
    let mut clipboard = MemoryClipboard::default();
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .unwrap();
    assert!(state.horizontal_offset() > 0);
    state.toggle_soft_wrap();
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::Home, KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .unwrap();
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .unwrap();
    assert!(state.display_window().unwrap().start_cell > 0);
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("012345678901234567890123456789")
    );
}

#[test]
fn dragging_past_the_right_edge_includes_the_last_visible_character() {
    let mut state = ArkLogState::new().unwrap();
    state.append_lines(["12345678"]).unwrap();
    state.set_viewport_size(8, 2);
    let area = Rect::new(1, 4, 8, 2);
    state
        .handle_log_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 1, 4), area)
        .unwrap();
    state
        .handle_log_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 12, 4), area)
        .unwrap();
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("12345678")
    );
}

#[test]
fn scrolling_to_the_bottom_keeps_an_active_selection_paused() {
    let mut state = ArkLogState::new().unwrap();
    state.append_lines(["first", "second", "third"]).unwrap();
    state.set_viewport_size(20, 2);
    state
        .handle_log_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
            &mut MemoryClipboard::default(),
        )
        .unwrap();
    assert!(state
        .handle_log_mouse(
            mouse(MouseEventKind::ScrollDown, 2, 5),
            Rect::new(1, 4, 20, 2)
        )
        .unwrap());
    assert!(!state.is_following_latest());
    let before = state.display_window().unwrap();
    state.append_lines(["newest"]).unwrap();
    assert_eq!(state.display_window().unwrap(), before);
}

#[test]
fn resize_preserves_source_selection_and_view_changes_or_following_clear_it() {
    use arklog::LogTab;
    let mut state = ArkLogState::new().unwrap();
    state.append_lines(["first\t中", "second"]).unwrap();
    let mut clipboard = MemoryClipboard::default();
    let select_all = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    state.handle_log_key(select_all, &mut clipboard).unwrap();
    state.set_viewport_size(80, 20);
    state.toggle_soft_wrap();
    assert_eq!(
        state.selected_log_text().unwrap().as_deref(),
        Some("first\t中\nsecond")
    );
    state.select_tab(LogTab::FaultLog);
    assert!(state.log_selection().is_none());
    state.select_tab(LogTab::HiLog);
    state.handle_log_key(select_all, &mut clipboard).unwrap();
    state.follow_latest();
    assert!(state.log_selection().is_none());
    assert!(state.is_following_latest());
}
