use super::TerminalApp;
use arklog::{
    AppCommand, CommandContext, CommandKeymap, InputMode, LogTab, NavigationAction,
    NavigationContext, NavigationKeymap, OverlayMode, TextInput, TextInputView,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

impl TerminalApp {
    pub(super) fn input_view(&self) -> TextInputView<'_> {
        let mut input = self.input.view();
        if self.input_mode == InputMode::Filter
            && self.overlay == OverlayMode::None
            && self.controller.state().tab() == LogTab::HiLog
        {
            input.suggestion = self.controller.state().filter_suggestion(input);
        }
        input
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        if self.browsing_hilog() {
            match self
                .controller
                .state_mut()
                .handle_log_key(key, &mut self.clipboard)
            {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    self.record(Err(error));
                    return;
                }
            }
        }
        let command_context = if self.overlay == OverlayMode::Devices {
            CommandContext::Devices
        } else if self.controller.state().tab() == LogTab::FaultLog {
            CommandContext::FaultLog
        } else {
            CommandContext::HiLog
        };
        if let Some(command) = CommandKeymap::resolve(key, command_context) {
            self.handle_command(command);
            return;
        }
        let navigation_context = if self.overlay == OverlayMode::Devices {
            NavigationContext::Devices
        } else {
            match self.input_mode {
                InputMode::Filter => NavigationContext::FilterInput,
                InputMode::Find => NavigationContext::FindInput,
                InputMode::Normal if self.controller.state().tab() == LogTab::HiLog => {
                    NavigationContext::HiLog
                }
                InputMode::Normal => NavigationContext::FaultLog,
            }
        };
        if let Some(action) = NavigationKeymap::resolve(key, navigation_context) {
            self.handle_navigation(action);
            return;
        }
        if self.overlay == OverlayMode::Devices {
            if key.code == KeyCode::Esc {
                self.overlay = OverlayMode::None;
            }
            return;
        }
        if self.input_mode != InputMode::Normal {
            self.handle_input_key(key);
            return;
        }
        match key.code {
            KeyCode::Tab | KeyCode::BackTab
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.controller.state_mut().next_tab();
            }
            KeyCode::End => self.controller.state_mut().follow_latest(),
            KeyCode::Up => self.move_up(1),
            KeyCode::Down => self.move_down(1),
            KeyCode::PageUp if self.controller.state().tab() == LogTab::FaultLog => {
                self.fault_scroll = self.fault_scroll.saturating_sub(10);
            }
            KeyCode::PageDown if self.controller.state().tab() == LogTab::FaultLog => {
                self.scroll_fault_down(10);
            }
            KeyCode::PageUp => self.move_up(10),
            KeyCode::PageDown => self.move_down(10),
            _ => {}
        }
    }

    fn handle_navigation(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::ScrollLeft => self.controller.state_mut().scroll_horizontal_left(),
            NavigationAction::ScrollRight => {
                let result = self
                    .controller
                    .state_mut()
                    .scroll_horizontal_right()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
            NavigationAction::ResetHorizontal => self.controller.state_mut().reset_horizontal(),
            NavigationAction::PreviousDevice => {
                let result = self.controller.previous_device();
                self.record(result);
            }
            NavigationAction::NextDevice => {
                let result = self.controller.next_device();
                self.record(result);
            }
            NavigationAction::OlderFilter => {
                let current = self.input.text().to_string();
                if let Some(query) = self.controller.state_mut().older_filter_query(&current) {
                    self.input.replace_all(&query);
                }
            }
            NavigationAction::NewerFilter => {
                let current = self.input.text().to_string();
                if let Some(query) = self.controller.state_mut().newer_filter_query(&current) {
                    self.input.replace_all(&query);
                }
            }
            NavigationAction::ToggleSoftWrap => self.controller.state_mut().toggle_soft_wrap(),
        }
    }

    fn handle_command(&mut self, command: AppCommand) {
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record_command(&format!("{command:?}"));
        }
        match command {
            AppCommand::Quit => self.should_quit = true,
            AppCommand::SelectLogTab(tab) => {
                self.controller.state_mut().select_tab(tab);
                self.overlay = OverlayMode::None;
                self.input_mode = InputMode::Normal;
                self.input = TextInput::new("");
            }
            AppCommand::ToggleStream => self.toggle_stream(),
            AppCommand::Refresh => self.refresh(),
            AppCommand::ToggleDevices => {
                self.overlay = if self.overlay == OverlayMode::Devices {
                    OverlayMode::None
                } else {
                    OverlayMode::Devices
                };
            }
            AppCommand::EditFilter if self.controller.state().tab() == LogTab::HiLog => {
                self.controller
                    .state_mut()
                    .cancel_filter_history_navigation();
                self.input_mode = InputMode::Filter;
                self.input = TextInput::new(self.controller.state().filter_query());
            }
            AppCommand::EditFilter => {}
            AppCommand::Find => {
                self.input_mode = InputMode::Find;
                self.input = TextInput::new(self.controller.state().find_query());
            }
            AppCommand::NextMatch => {
                let result = self
                    .controller
                    .state_mut()
                    .next_find()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
            AppCommand::PreviousMatch => {
                let result = self
                    .controller
                    .state_mut()
                    .previous_find()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
            AppCommand::FollowLatest => self.controller.state_mut().follow_latest(),
            AppCommand::Clear => {
                let started = Instant::now();
                let result = self
                    .controller
                    .state_mut()
                    .clear()
                    .map_err(|error| error.to_string());
                if let Some(diagnostics) = &self.diagnostics {
                    diagnostics.record_clear_finished(&result, started.elapsed());
                }
                self.trace_clear_frame = true;
                self.record(result);
            }
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab if self.input_mode == InputMode::Filter && key.modifiers.is_empty() => {
                if let Some(suffix) = self.controller.state().filter_suggestion(self.input.view()) {
                    self.input.insert_text(suffix);
                }
            }
            KeyCode::Esc => {
                self.controller
                    .state_mut()
                    .cancel_filter_history_navigation();
                self.input_mode = InputMode::Normal;
                self.input = TextInput::new("");
            }
            KeyCode::Enter if self.input_mode == InputMode::Filter => {
                let query = self.input.text().to_string();
                let result = self
                    .controller
                    .state_mut()
                    .apply_filter(query)
                    .and_then(|_| self.persist_history());
                self.record(result);
                self.input_mode = InputMode::Normal;
                self.input = TextInput::new("");
            }
            KeyCode::Enter => {
                self.apply_or_advance_find(key.modifiers.contains(KeyModifiers::SHIFT))
            }
            _ => match self.input.handle_key(key, &mut self.clipboard) {
                Ok(_) => {}
                Err(error) => self.action_status.record_action(Err(error)),
            },
        }
    }

    fn apply_or_advance_find(&mut self, backwards: bool) {
        let result = if self.input.text() != self.controller.state().find_query() {
            self.controller
                .state_mut()
                .set_find(self.input.text().to_string())
        } else if backwards {
            self.controller.state_mut().previous_find()
        } else {
            self.controller.state_mut().next_find()
        };
        self.record(result.map_err(|error| error.to_string()));
    }
}
