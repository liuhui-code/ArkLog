use arklog::{
    AppCommand, CommandContext, CommandKeymap, NavigationAction, NavigationContext,
    NavigationKeymap,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn character_commands_require_the_control_modifier() {
    let cases = [
        ('q', AppCommand::Quit),
        ('s', AppCommand::ToggleStream),
        ('r', AppCommand::Refresh),
        ('d', AppCommand::ToggleDevices),
        ('e', AppCommand::EditFilter),
        ('f', AppCommand::Find),
        ('n', AppCommand::NextMatch),
        ('p', AppCommand::PreviousMatch),
        ('g', AppCommand::FollowLatest),
        ('l', AppCommand::Clear),
    ];

    for (character, command) in cases {
        let bare = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        let controlled = KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL);
        assert_eq!(CommandKeymap::resolve(bare, CommandContext::HiLog), None);
        assert_eq!(
            CommandKeymap::resolve(controlled, CommandContext::HiLog),
            Some(command)
        );
    }
}

#[test]
fn hilog_commands_are_disabled_outside_the_hilog_workspace() {
    for character in ['e', 'f', 'n', 'p', 'g', 'l'] {
        let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL);
        assert_eq!(CommandKeymap::resolve(key, CommandContext::FaultLog), None);
        assert_eq!(CommandKeymap::resolve(key, CommandContext::Devices), None);
    }
}

#[test]
fn editing_and_terminal_copy_shortcuts_are_not_claimed_as_global_commands() {
    let windows_copy = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    for context in [
        CommandContext::HiLog,
        CommandContext::FaultLog,
        CommandContext::Devices,
    ] {
        assert_eq!(CommandKeymap::resolve(windows_copy, context), None);
    }

    let terminal_copy = KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let macos_copy = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SUPER);
    assert_eq!(
        CommandKeymap::resolve(terminal_copy, CommandContext::HiLog),
        None
    );
    assert_eq!(
        CommandKeymap::resolve(macos_copy, CommandContext::HiLog),
        None
    );
}

#[test]
fn horizontal_keys_follow_the_focused_workspace_without_stealing_input_editing() {
    let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    let home = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);

    assert_eq!(
        NavigationKeymap::resolve(left, NavigationContext::HiLog),
        Some(NavigationAction::ScrollLeft)
    );
    assert_eq!(
        NavigationKeymap::resolve(right, NavigationContext::HiLog),
        Some(NavigationAction::ScrollRight)
    );
    assert_eq!(
        NavigationKeymap::resolve(home, NavigationContext::HiLog),
        Some(NavigationAction::ResetHorizontal)
    );
    assert_eq!(
        NavigationKeymap::resolve(left, NavigationContext::Devices),
        Some(NavigationAction::PreviousDevice)
    );
    assert_eq!(
        NavigationKeymap::resolve(right, NavigationContext::Devices),
        Some(NavigationAction::NextDevice)
    );
    for context in [NavigationContext::FilterInput, NavigationContext::FindInput] {
        assert_eq!(NavigationKeymap::resolve(left, context), None);
        assert_eq!(NavigationKeymap::resolve(right, context), None);
        assert_eq!(NavigationKeymap::resolve(home, context), None);
    }
}

#[test]
fn filter_history_arrows_are_scoped_to_the_filter_editor() {
    let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        NavigationKeymap::resolve(up, NavigationContext::FilterInput),
        Some(NavigationAction::OlderFilter)
    );
    assert_eq!(
        NavigationKeymap::resolve(down, NavigationContext::FilterInput),
        Some(NavigationAction::NewerFilter)
    );
    for context in [
        NavigationContext::FindInput,
        NavigationContext::HiLog,
        NavigationContext::FaultLog,
        NavigationContext::Devices,
    ] {
        assert_eq!(NavigationKeymap::resolve(up, context), None);
        assert_eq!(NavigationKeymap::resolve(down, context), None);
    }
}

#[test]
fn soft_wrap_shortcut_is_available_only_in_the_hilog_body() {
    let toggle = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert_eq!(
        NavigationKeymap::resolve(toggle, NavigationContext::HiLog),
        Some(NavigationAction::ToggleSoftWrap)
    );
    for context in [
        NavigationContext::FilterInput,
        NavigationContext::FindInput,
        NavigationContext::FaultLog,
        NavigationContext::Devices,
    ] {
        assert_eq!(NavigationKeymap::resolve(toggle, context), None);
    }
}
