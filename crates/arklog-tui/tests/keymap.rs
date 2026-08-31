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
