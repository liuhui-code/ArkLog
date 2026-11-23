use super::TerminalApp;
use arklog::{ArkLogController, TextInput};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn app_with_history() -> TerminalApp {
    let mut controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    controller
        .state_mut()
        .apply_filter("error|timeout")
        .unwrap();
    controller.state_mut().apply_filter("").unwrap();
    let mut app = TerminalApp::new(controller, None, None);
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.input = TextInput::new("err");
    app
}

#[test]
fn tab_accepts_only_the_preview_and_enter_does_not_accept_it_implicitly() {
    let mut app = app_with_history();
    assert_eq!(app.input_view().suggestion, Some("or|timeout"));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "err");

    let mut app = app_with_history();
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "error|timeout");
    assert_eq!(app.controller.state().filter_query(), "");
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.input.text(), "err");
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "error|timeout");
}

#[test]
fn applying_a_filter_saves_history_and_a_new_app_restores_suggestions_only() {
    let path =
        std::env::temp_dir().join(format!("arklog-editor-history-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut first = TerminalApp::new(controller, None, None);
    first.restore_history(Some(path.clone()));
    first.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    first.input = TextInput::new("error|timeout");
    first.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(path.exists());
    let saved = std::fs::read(&path).unwrap();

    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut second = TerminalApp::new(controller, None, None);
    second.restore_history(Some(path.clone()));
    second.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    second.input = TextInput::new("err");
    assert_eq!(second.input_view().suggestion, Some("or|timeout"));
    assert_eq!(second.controller.state().filter_query(), "");
    second.input = TextInput::new("[");
    second.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(second.action_status.error().is_some());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn previews_are_filter_only_and_history_navigation_restores_the_draft() {
    use arklog::{InputMode, OverlayMode};
    let mut app = app_with_history();
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "error|timeout");
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "err");
    assert_eq!(app.input_view().suggestion, Some("or|timeout"));
    app.input_mode = InputMode::Find;
    assert_eq!(app.input_view().suggestion, None);
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "err");
    app.input_mode = InputMode::Filter;
    app.overlay = OverlayMode::Devices;
    assert_eq!(app.input_view().suggestion, None);
}

#[test]
fn damaged_history_is_nonfatal_and_a_successful_apply_can_repair_it() {
    let path = std::env::temp_dir().join(format!(
        "arklog-damaged-history-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, "broken json").unwrap();
    let mut app = app_with_history();
    app.restore_history(Some(path.clone()));
    assert!(app
        .action_status
        .error()
        .unwrap()
        .contains("Regex history unavailable"));
    assert_eq!(app.input_view().suggestion, Some("or|timeout"));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "err");
    assert!(app.action_status.error().is_none());
    assert!(arklog::QueryHistory::load(&path).is_ok());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn level_queries_use_existing_apply_history_and_tab_completion_without_accepting_the_hint() {
    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut app = TerminalApp::new(controller, None, None);
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(app.input_view().suggestion, None);
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "");
    assert!(app.action_status.error().is_none());
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.controller
        .state_mut()
        .append_lines(["0 I timeout", "1 W Timeout", "2 F unrelated"])
        .unwrap();
    app.input = TextInput::new("level:WARN timeout");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(app.action_status.error().is_none());
    assert_eq!(
        app.controller.state_mut().visible_window(10).unwrap(),
        ["1 W Timeout"]
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.input = TextInput::new("level:W");
    assert_eq!(app.input_view().suggestion, Some("ARN timeout"));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "level:WARN timeout");
    app.input = TextInput::new("level:TYPO");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(app.action_status.error().is_some());
    assert_eq!(
        app.controller.state_mut().older_filter_query(""),
        Some("level:WARN timeout".into())
    );
}

#[test]
fn builtin_level_completion_uses_tab_undo_and_enter_without_history_or_extra_keys() {
    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut app = TerminalApp::new(controller, None, None);
    app.controller
        .state_mut()
        .append_lines(["0 I info", "1 W warning"])
        .unwrap();
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.input = TextInput::new("level:W");
    assert_eq!(app.input_view().suggestion, Some("ARN"));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.input.text(), "level:WARN");
    assert_eq!(app.controller.state().filter_query(), "");
    assert_eq!(app.controller.state_mut().older_filter_query(""), None);
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.input.text(), "level:W");
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "level:WARN");
    assert_eq!(
        app.controller.state_mut().visible_window(10).unwrap(),
        ["1 W warning"]
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.input = TextInput::new("lev");
    assert_eq!(app.input_view().suggestion, Some("el:WARN"));
    app.input_mode = arklog::InputMode::Find;
    assert_eq!(app.input_view().suggestion, None);
    app.input_mode = arklog::InputMode::Filter;
    app.overlay = arklog::OverlayMode::Devices;
    assert_eq!(app.input_view().suggestion, None);
    app.overlay = arklog::OverlayMode::None;
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.controller.state().filter_query(), "lev");
}
