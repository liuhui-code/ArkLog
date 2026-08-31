use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Quit,
    ToggleStream,
    Refresh,
    ToggleDevices,
    EditFilter,
    Find,
    NextMatch,
    PreviousMatch,
    FollowLatest,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandContext {
    HiLog,
    FaultLog,
    Devices,
}

pub struct CommandKeymap;

impl CommandKeymap {
    pub fn resolve(key: KeyEvent, context: CommandContext) -> Option<AppCommand> {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            return None;
        }
        let KeyCode::Char(character) = key.code else {
            return None;
        };
        let command = match character.to_ascii_lowercase() {
            'q' => Some(AppCommand::Quit),
            's' => Some(AppCommand::ToggleStream),
            'r' => Some(AppCommand::Refresh),
            'd' => Some(AppCommand::ToggleDevices),
            'e' => Some(AppCommand::EditFilter),
            'f' => Some(AppCommand::Find),
            'n' => Some(AppCommand::NextMatch),
            'p' => Some(AppCommand::PreviousMatch),
            'g' => Some(AppCommand::FollowLatest),
            'l' => Some(AppCommand::Clear),
            _ => None,
        }?;
        let hilog_only = matches!(
            command,
            AppCommand::EditFilter
                | AppCommand::Find
                | AppCommand::NextMatch
                | AppCommand::PreviousMatch
                | AppCommand::FollowLatest
                | AppCommand::Clear
        );
        if hilog_only && context != CommandContext::HiLog {
            None
        } else {
            Some(command)
        }
    }
}
