use super::{RedrawState, TerminalApp};
use crate::should_relaunch_in_windows_terminal;
use arklog::{ArkLogController, InputMode, LogTab, OverlayMode, TextInput};
use ratatui::{
    crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    layout::Rect,
};

#[test]
#[cfg(unix)]
fn leaving_log_selection_restores_terminal_mouse_handling_even_on_error() {
    let mut output = Vec::new();
    let fail = |output: &mut Vec<u8>| -> std::io::Result<()> {
        let mut capture = super::MouseCapture::new(output);
        capture.set_enabled(true)?;
        capture.set_enabled(true)?;
        Err(std::io::Error::other("simulated event failure"))
    };
    assert!(fail(&mut output).is_err());
    let commands = String::from_utf8(output).unwrap();
    assert_eq!(commands.matches("\u{1b}[?1000h").count(), 1);
    assert_eq!(commands.matches("\u{1b}[?1000l").count(), 1);
}

#[test]
fn mouse_selection_reaches_hilog_but_not_query_editors_or_device_overlays() {
    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut app = TerminalApp::new(controller, None, None);
    app.controller
        .state_mut()
        .append_lines(["first line"])
        .unwrap();
    app.controller.state_mut().set_viewport_size(78, 15);
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 4,
        modifiers: KeyModifiers::NONE,
    };
    let area = Rect::new(0, 0, 80, 20);
    app.input_mode = InputMode::Filter;
    assert!(!app.handle_mouse(click, area));
    app.input_mode = InputMode::Normal;
    app.overlay = OverlayMode::Devices;
    assert!(!app.handle_mouse(click, area));
    app.overlay = OverlayMode::None;
    assert!(app.handle_mouse(click, area));
    let release = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 6,
        ..click
    };
    assert!(app.handle_mouse(release, area));
    assert_eq!(
        app.controller
            .state_mut()
            .selected_log_text()
            .unwrap()
            .as_deref(),
        Some("first")
    );
}

#[test]
fn selecting_log_text_with_shift_arrows_pauses_following_live_logs() {
    let controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    let mut app = TerminalApp::new(controller, None, None);
    app.controller
        .state_mut()
        .append_lines(["first log", "second log"])
        .unwrap();
    app.controller.state_mut().set_viewport_size(80, 10);

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));

    assert!(
        !app.controller.state().is_following_latest(),
        "selecting text must hold the viewport still"
    );
}

#[test]
fn shift_tab_switches_between_the_log_views() {
    let controller =
        ArkLogController::new("hdc-must-not-be-launched", Vec::new()).expect("controller");
    let mut app = TerminalApp::new(controller, None, None);

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));

    assert_eq!(app.controller.state().tab(), LogTab::FaultLog);
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.controller.state().tab(), LogTab::HiLog);
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.controller.state().tab(), LogTab::FaultLog);
}

#[test]
fn modified_terminal_tab_shortcuts_do_not_switch_log_views() {
    let controller =
        ArkLogController::new("hdc-must-not-be-launched", Vec::new()).expect("controller");
    let mut app = TerminalApp::new(controller, None, None);

    for code in [KeyCode::Tab, KeyCode::BackTab] {
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyModifiers::ALT,
            KeyModifiers::ALT | KeyModifiers::SHIFT,
            KeyModifiers::SUPER,
        ] {
            app.handle_key(KeyEvent::new(code, modifiers));
            assert_eq!(app.controller.state().tab(), LogTab::HiLog);
        }
    }
}

#[test]
fn modified_function_keys_and_tab_in_editors_preserve_navigation_and_drafts() {
    let controller =
        ArkLogController::new("hdc-must-not-be-launched", Vec::new()).expect("controller");
    let mut app = TerminalApp::new(controller, None, None);

    for mode in [InputMode::Normal, InputMode::Filter, InputMode::Find] {
        for overlay in [OverlayMode::None, OverlayMode::Devices] {
            app.input_mode = mode;
            app.overlay = overlay;
            app.input = TextInput::new("unapplied draft");
            for function in [1, 2] {
                for modifier in [
                    KeyModifiers::CONTROL,
                    KeyModifiers::ALT,
                    KeyModifiers::SHIFT,
                    KeyModifiers::SUPER,
                ] {
                    app.handle_key(KeyEvent::new(KeyCode::F(function), modifier));
                    assert_eq!(app.controller.state().tab(), LogTab::HiLog);
                    assert_eq!(app.input_mode, mode);
                    assert_eq!(app.overlay, overlay);
                    assert_eq!(app.input.text(), "unapplied draft");
                }
            }
            if mode != InputMode::Normal || overlay != OverlayMode::None {
                for code in [KeyCode::Tab, KeyCode::BackTab] {
                    for modifier in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
                        app.handle_key(KeyEvent::new(code, modifier));
                        assert_eq!(app.controller.state().tab(), LogTab::HiLog);
                        assert_eq!(app.input_mode, mode);
                        assert_eq!(app.overlay, overlay);
                        assert_eq!(app.input.text(), "unapplied draft");
                    }
                }
            }
        }
    }
}

#[test]
fn function_keys_select_log_views_even_while_editing_or_browsing_devices() {
    let mut controller =
        ArkLogController::new("hdc-must-not-be-launched", Vec::new()).expect("controller");
    controller
        .state_mut()
        .append_lines(["kept log"])
        .expect("log");
    controller.state_mut().apply_filter("kept").expect("filter");
    controller.state_mut().set_find("log").expect("find");
    let mut app = TerminalApp::new(controller, None, None);
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    app.input = TextInput::new("unapplied draft");

    app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));

    assert_eq!(app.controller.state().tab(), LogTab::FaultLog);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.controller.state().raw_count(), 1);
    assert_eq!(app.controller.state().filter_query(), "kept");
    assert_eq!(app.controller.state().find_query(), "log");
    for kind in [KeyEventKind::Press, KeyEventKind::Repeat] {
        let mut repeated = KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE);
        repeated.kind = kind;
        app.handle_key(repeated);
        assert_eq!(app.controller.state().tab(), LogTab::FaultLog);
    }

    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(app.overlay, OverlayMode::Devices);
    app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    assert_eq!(app.controller.state().tab(), LogTab::HiLog);
    assert_eq!(app.overlay, OverlayMode::None);

    app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
    app.input = TextInput::new("unapplied find draft");
    app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.controller.state().tab(), LogTab::FaultLog);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.controller.state().find_query(), "log");
    assert_eq!(app.controller.state().filter_query(), "kept");
    assert_eq!(app.controller.state().raw_count(), 1);
}

#[test]
fn windows_terminal_relaunch_requires_an_explicit_request() {
    assert!(!should_relaunch_in_windows_terminal(false, true, false, 1));
    assert!(should_relaunch_in_windows_terminal(true, true, false, 1));
    assert!(!should_relaunch_in_windows_terminal(true, true, true, 1));
    assert!(!should_relaunch_in_windows_terminal(true, true, false, 2));
    assert!(!should_relaunch_in_windows_terminal(true, false, false, 1));
}

#[test]
fn redraw_state_coalesces_changes_and_stays_clean_while_idle() {
    let mut redraw = RedrawState::new();

    assert!(redraw.take());
    assert!(!redraw.take());
    redraw.mark();
    redraw.mark();
    assert!(redraw.take());
    assert!(!redraw.take());
}

#[test]
fn ctrl_l_records_when_clearing_the_live_view_finishes() {
    let path = std::env::temp_dir().join(format!("arklog-clear-test-{}.log", std::process::id()));
    let log = arklog::ExecutionLog::start(&path, 4096).expect("execution log");
    let mut controller =
        ArkLogController::new("hdc-must-not-be-launched", Vec::new()).expect("controller");
    controller
        .state_mut()
        .append_lines(["private log body"])
        .expect("initial logs");
    let mut app = TerminalApp::new(controller, None, Some(log));

    app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));

    assert_eq!(app.controller.state().raw_count(), 0);
    drop(app);
    let content = std::fs::read_to_string(&path).expect("clear diagnostics");
    std::fs::remove_file(path).expect("remove test log");
    assert!(content.contains("event=clear_finished"), "{content}");
    assert!(content.contains("result=\"ok\""), "{content}");
    assert!(content.contains("elapsed_ms="), "{content}");
    assert!(!content.contains("private log body"), "{content}");
}
