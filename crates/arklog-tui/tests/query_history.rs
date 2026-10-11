use arklog::{Clipboard, QueryHistory, TextInput};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

struct NoClipboard;

impl Clipboard for NoClipboard {
    fn get_text(&mut self) -> Result<String, String> {
        Ok(String::new())
    }

    fn set_text(&mut self, _text: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn successful_queries_are_exactly_deduplicated_and_restore_the_draft() {
    let mut history = QueryHistory::new();
    assert!(history.record_applied("alpha"));
    assert!(history.record_applied("beta"));
    assert!(history.record_applied("alpha"));

    assert_eq!(history.older("draft"), Some("alpha".to_string()));
    assert_eq!(history.older("alpha"), Some("beta".to_string()));
    assert_eq!(history.older("beta"), None);
    assert_eq!(history.newer("beta"), Some("alpha".to_string()));
    assert_eq!(history.newer("alpha"), Some("draft".to_string()));
    assert_eq!(history.newer("draft"), None);
}

#[test]
fn history_rejects_empty_or_oversized_queries_and_evicts_the_oldest_of_fifty() {
    let mut history = QueryHistory::new();
    assert!(!history.record_applied(""));
    assert!(!history.record_applied(&"x".repeat(4_097)));
    for index in 0..51 {
        assert!(history.record_applied(&format!("query-{index:02}")));
    }

    let mut current = "draft".to_string();
    let mut visited = Vec::new();
    while let Some(query) = history.older(&current) {
        current = query.clone();
        visited.push(query);
    }

    assert_eq!(visited.len(), 50);
    assert_eq!(visited.first().map(String::as_str), Some("query-50"));
    assert_eq!(visited.last().map(String::as_str), Some("query-01"));
    assert!(!visited.iter().any(|query| query == "query-00"));
}

#[test]
fn edited_history_items_survive_navigation_and_use_the_text_input_undo_stack() {
    let mut history = QueryHistory::new();
    history.record_applied("older");
    history.record_applied("newer");
    let mut input = TextInput::new("draft");

    let newer = history.older(input.text()).expect("newest history");
    input.replace_all(&newer);
    input.insert_text("-edited");
    let older = history.older(input.text()).expect("older history");
    input.replace_all(&older);
    let edited = history.newer(input.text()).expect("edited newer history");
    input.replace_all(&edited);
    assert_eq!(input.text(), "newer-edited");

    input
        .handle_key(
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            &mut NoClipboard,
        )
        .expect("undo history replacement");
    assert_eq!(input.text(), "older");
}

#[test]
fn history_does_not_trim_or_case_fold_regular_expressions() {
    let mut history = QueryHistory::new();
    history.record_applied("Error");
    history.record_applied("error");
    history.record_applied(" Error ");

    assert_eq!(history.older(""), Some(" Error ".to_string()));
    assert_eq!(history.older(" Error "), Some("error".to_string()));
    assert_eq!(history.older("error"), Some("Error".to_string()));
}
