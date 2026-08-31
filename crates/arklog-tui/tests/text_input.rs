use arklog::{Clipboard, TextInput};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
struct MemoryClipboard {
    text: String,
}

impl Clipboard for MemoryClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        Ok(self.text.clone())
    }

    fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.text = text.to_string();
        Ok(())
    }
}

#[test]
fn primary_a_selects_the_whole_draft_for_replacement_on_each_desktop() {
    for modifier in [KeyModifiers::CONTROL, KeyModifiers::SUPER] {
        let mut input = TextInput::new("task-123");
        let mut clipboard = MemoryClipboard::default();

        input
            .handle_key(KeyEvent::new(KeyCode::Char('a'), modifier), &mut clipboard)
            .expect("select all");
        assert_eq!(input.view().selection, Some((0, "task-123".len())));
        input
            .handle_key(
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
                &mut clipboard,
            )
            .expect("replace selection");

        assert_eq!(input.text(), "x");
        assert_eq!(input.cursor_chars(), 1);
    }
}

#[test]
fn copy_cut_and_paste_operate_on_the_selected_input_text() {
    let mut input = TextInput::new("error|warning");
    let mut clipboard = MemoryClipboard::default();

    for character in ['a', 'c'] {
        input
            .handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL),
                &mut clipboard,
            )
            .expect("copy selected draft");
    }
    assert_eq!(input.text(), "error|warning");
    assert_eq!(clipboard.text, "error|warning");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("cut selected draft");
    assert_eq!(input.text(), "");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("paste draft");
    assert_eq!(input.text(), "error|warning");
}

#[test]
fn undo_and_redo_restore_text_edits_with_desktop_shortcuts() {
    let mut input = TextInput::new("needle");
    let mut clipboard = MemoryClipboard::default();

    for character in ['a', 'x', 'z'] {
        input
            .handle_key(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL),
                &mut clipboard,
            )
            .expect("edit then undo");
    }
    assert_eq!(input.text(), "needle");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("redo with Ctrl+Y");
    assert_eq!(input.text(), "");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("undo again");
    input
        .handle_key(
            KeyEvent::new(
                KeyCode::Char('z'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            &mut clipboard,
        )
        .expect("redo with Ctrl+Shift+Z");
    assert_eq!(input.text(), "");
}

#[test]
fn arrows_home_and_end_move_or_extend_a_unicode_safe_selection() {
    let mut input = TextInput::new("ab中d");
    let mut clipboard = MemoryClipboard::default();

    input
        .handle_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("move left");
    input
        .handle_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .expect("select unicode character");
    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .expect("replace selection");
    assert_eq!(input.text(), "abXd");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("home");
    assert_eq!(input.cursor_chars(), 0);
    input
        .handle_key(
            KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("end");
    assert_eq!(input.cursor_chars(), 4);
}

#[test]
fn delete_keys_remove_characters_or_words_without_splitting_unicode() {
    let mut clipboard = MemoryClipboard::default();
    let mut input = TextInput::new("ab中");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("backspace unicode");
    input
        .handle_key(
            KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("home");
    input
        .handle_key(
            KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("delete character");
    assert_eq!(input.text(), "b");

    let mut words = TextInput::new("one two 三");
    for _ in 0..2 {
        words
            .handle_key(
                KeyEvent::new(KeyCode::Backspace, KeyModifiers::CONTROL),
                &mut clipboard,
            )
            .expect("delete previous word");
    }
    assert_eq!(words.text(), "one ");
    words
        .handle_key(
            KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
            &mut clipboard,
        )
        .expect("home");
    words
        .handle_key(
            KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("delete next word");
    assert_eq!(words.text(), "");
}

#[test]
fn primary_arrows_move_by_words_and_shift_extends_the_selection() {
    let mut input = TextInput::new("one two");
    let mut clipboard = MemoryClipboard::default();

    input
        .handle_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("previous word");
    assert_eq!(input.cursor_chars(), 4);
    input
        .handle_key(
            KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            &mut clipboard,
        )
        .expect("select next word");
    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("copy selected word");

    assert_eq!(clipboard.text, "two");
}

#[test]
fn macos_command_moves_by_line_while_option_moves_by_word() {
    let mut input = TextInput::new("one two");
    let mut clipboard = MemoryClipboard::default();

    input
        .handle_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::ALT),
            &mut clipboard,
        )
        .expect("Option+Left");
    assert_eq!(input.cursor_chars(), 4);
    input
        .handle_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::SUPER),
            &mut clipboard,
        )
        .expect("Command+Left");
    assert_eq!(input.cursor_chars(), 0);
    input
        .handle_key(
            KeyEvent::new(KeyCode::Delete, KeyModifiers::SUPER),
            &mut clipboard,
        )
        .expect("Command+Delete");
    assert_eq!(input.text(), "");
}

#[test]
fn paste_stays_single_line_utf8_and_within_the_query_memory_budget() {
    let mut input = TextInput::new("");
    let mut clipboard = MemoryClipboard {
        text: format!("a\nb\rc\0{}", "中".repeat(2_000)),
    };

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            &mut clipboard,
        )
        .expect("bounded paste");

    assert!(input.text().starts_with("a b c"));
    assert!(!input.text().chars().any(char::is_control));
    assert!(input.text().len() <= 4 * 1024);
    assert!(input.text().is_char_boundary(input.text().len()));
}
