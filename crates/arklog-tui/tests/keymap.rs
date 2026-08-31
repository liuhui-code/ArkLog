use arklog::{AppCommand, CommandContext, CommandKeymap};
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
