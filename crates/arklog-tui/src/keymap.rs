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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationAction {
    ScrollLeft,
    ScrollRight,
    ResetHorizontal,
    PreviousDevice,
    NextDevice,
    OlderFilter,
    NewerFilter,
    ToggleSoftWrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationContext {
    HiLog,
    FaultLog,
    Devices,
    FilterInput,
    FindInput,
}

pub struct NavigationKeymap;

impl NavigationKeymap {
    pub fn resolve(key: KeyEvent, context: NavigationContext) -> Option<NavigationAction> {
        if context == NavigationContext::HiLog
            && key.modifiers == KeyModifiers::CONTROL
            && matches!(key.code, KeyCode::Char('w' | 'W'))
        {
            return Some(NavigationAction::ToggleSoftWrap);
        }
        if !key.modifiers.is_empty() {
            return None;
        }
        match (context, key.code) {
            (NavigationContext::HiLog, KeyCode::Left) => Some(NavigationAction::ScrollLeft),
            (NavigationContext::HiLog, KeyCode::Right) => Some(NavigationAction::ScrollRight),
            (NavigationContext::HiLog, KeyCode::Home) => Some(NavigationAction::ResetHorizontal),
            (NavigationContext::Devices, KeyCode::Left) => Some(NavigationAction::PreviousDevice),
            (NavigationContext::Devices, KeyCode::Right) => Some(NavigationAction::NextDevice),
            (NavigationContext::FilterInput, KeyCode::Up) => Some(NavigationAction::OlderFilter),
            (NavigationContext::FilterInput, KeyCode::Down) => Some(NavigationAction::NewerFilter),
            _ => None,
        }
    }
}

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
