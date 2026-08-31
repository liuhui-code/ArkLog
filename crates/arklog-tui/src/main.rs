use std::env;
use std::io;
use std::time::Duration;

use arklog::{
    render_app, run_memory_probe, ActionStatus, AppCommand, AppView, ArkLogController,
    CommandContext, CommandKeymap, InputMode, LogTab, OverlayMode, MEMORY_BUDGET_BYTES,
};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    DefaultTerminal,
};

const MAX_LOG_BATCHES_PER_TICK: usize = 8;

fn main() -> io::Result<()> {
    if env::args().nth(1).as_deref() == Some("--memory-probe") {
        return run_memory_budget_probe();
    }
    let executable = env::var("ARKLOG_HDC_PATH")
        .or_else(|_| env::var("ARKLOG_HDC"))
        .unwrap_or_else(|_| "hdc".to_string());
    let mut controller = ArkLogController::new(executable, Vec::new()).map_err(io::Error::other)?;
    let startup_error = controller.request_device_refresh().err();
    let mut terminal = ratatui::try_init()?;
    let result = TerminalApp::new(controller, startup_error).run(&mut terminal);
    ratatui::try_restore()?;
    result
}

fn run_memory_budget_probe() -> io::Result<()> {
    let line_count = env::args()
        .nth(2)
        .as_deref()
        .unwrap_or("100000")
        .parse::<u64>()
        .map_err(io::Error::other)?;
    let probe = run_memory_probe(line_count)?;
    println!(
        "lines={} visible={} find={} fault_log_bytes={} append_ms={} filter_ms={} find_ms={} retained_heap_bytes={} peak_rss_bytes={} budget_bytes={}",
        probe.raw_count,
        probe.visible_count,
        probe.find_count,
        probe.fault_log_bytes,
        probe.append_millis,
        probe.filter_millis,
        probe.find_millis,
        probe.retained_heap_bytes,
        probe.rss_bytes,
        MEMORY_BUDGET_BYTES
    );
    if probe.rss_bytes >= MEMORY_BUDGET_BYTES {
        return Err(io::Error::other(format!(
            "ArkLog exceeded the 50 MiB RSS budget: {} bytes",
            probe.rss_bytes
        )));
    }
    Ok(())
}

struct TerminalApp {
    controller: ArkLogController,
    input_mode: InputMode,
    overlay: OverlayMode,
    input_draft: String,
    action_status: ActionStatus,
    should_quit: bool,
    viewport_height: usize,
    fault_scroll: usize,
    start_when_device_ready: bool,
}

struct RedrawState {
    dirty: bool,
}

impl RedrawState {
    fn new() -> Self {
        Self { dirty: true }
    }

    fn mark(&mut self) {
        self.dirty = true;
    }

    fn mark_if(&mut self, changed: bool) {
        self.dirty |= changed;
    }

    fn take(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }
}

impl TerminalApp {
    fn new(controller: ArkLogController, action_error: Option<String>) -> Self {
        Self {
            controller,
            input_mode: InputMode::Normal,
            overlay: OverlayMode::None,
            input_draft: String::new(),
            action_status: ActionStatus::new(action_error),
            should_quit: false,
            viewport_height: 1,
            fault_scroll: 0,
            start_when_device_ready: true,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut redraw = RedrawState::new();
        while !self.should_quit {
            match self
                .controller
                .pump_log_batches_limited(MAX_LOG_BATCHES_PER_TICK)
            {
                Ok(batch_count) => redraw.mark_if(batch_count > 0),
                Err(error) => {
                    self.action_status.record_background(Err(error));
                    redraw.mark();
                }
            }
            match self.controller.pump_background_tasks_with_activity() {
                Ok(changed) => redraw.mark_if(changed),
                Err(error) => {
                    self.action_status.record_background(Err(error));
                    redraw.mark();
                }
            }
            if self.start_when_device_ready
                && self.controller.connection_state() != &arklog::ConnectionState::Refreshing
            {
                self.start_when_device_ready = false;
                if self.controller.connection_state() == &arklog::ConnectionState::Ready {
                    let start_result = self.controller.start_stream();
                    self.record(start_result);
                }
                redraw.mark();
            }
            if redraw.take() {
                let size = terminal.size()?;
                self.viewport_height = size.height.saturating_sub(10).max(1) as usize;
                let window_start = self
                    .controller
                    .state()
                    .visible_window_start(self.viewport_height);
                let lines = self
                    .controller
                    .state_mut()
                    .visible_window(self.viewport_height)?;
                terminal.draw(|frame| {
                    let state = self.controller.state();
                    let device = self.controller.selected_device();
                    render_app(
                        frame,
                        AppView {
                            device_id: device.map(|device| device.id.as_str()),
                            device_status: device
                                .map_or("No devices", |device| device.status.as_str()),
                            devices: self.controller.devices(),
                            selected_device: self.controller.selected_device_index(),
                            tab: state.tab(),
                            streaming: self.controller.stream_is_live(),
                            connection_status: self.controller.connection_state().message(),
                            stream_status: self.controller.stream_state().message(),
                            fault_status: self.controller.fault_state().message(),
                            action_error: self.action_status.error(),
                            raw_count: state.raw_count(),
                            visible_count: state.visible_count(),
                            following_latest: state.is_following_latest(),
                            filter_query: state.filter_query(),
                            active_filter: state.active_filter(),
                            filter_error: state.filter_error(),
                            find_query: state.find_query(),
                            find_status: state.find_status(),
                            current_find_visible_index: state.current_find_visible_index(),
                            window_start,
                            lines: &lines,
                            input_mode: self.input_mode,
                            overlay: self.overlay,
                            input_draft: &self.input_draft,
                            fault_result: self.controller.fault_result(),
                            selected_fault: self.controller.selected_fault(),
                            fault_scroll: self.fault_scroll,
                        },
                    );
                })?;
            }

            if event::poll(Duration::from_millis(30))? {
                match event::read()? {
                    Event::Key(key)
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                    {
                        self.handle_key(key);
                        redraw.mark();
                    }
                    Event::Resize(_, _) => redraw.mark(),
                    _ => {}
                }
            }
        }
        let stop_result = self.controller.stop_stream();
        self.record(stop_result);
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
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
        if self.overlay == OverlayMode::Devices {
            match key.code {
                KeyCode::Esc => self.overlay = OverlayMode::None,
                KeyCode::Left => {
                    let result = self.controller.previous_device();
                    self.record(result);
                }
                KeyCode::Right => {
                    let result = self.controller.next_device();
                    self.record(result);
                }
                _ => {}
            }
            return;
        }
        if self.input_mode != InputMode::Normal {
            self.handle_input_key(key);
            return;
        }
        match key.code {
            KeyCode::Tab => self.controller.state_mut().next_tab(),
            KeyCode::End => self.controller.state_mut().follow_latest(),
            KeyCode::Left => {
                let result = self.controller.previous_device();
                self.record(result);
            }
            KeyCode::Right => {
                let result = self.controller.next_device();
                self.record(result);
            }
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

    fn handle_command(&mut self, command: AppCommand) {
        match command {
            AppCommand::Quit => self.should_quit = true,
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
                self.input_mode = InputMode::Filter;
                self.input_draft = self.controller.state().filter_query().to_string();
            }
            AppCommand::EditFilter => {}
            AppCommand::Find => {
                self.input_mode = InputMode::Find;
                self.input_draft = self.controller.state().find_query().to_string();
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
                let result = self
                    .controller
                    .state_mut()
                    .clear()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_draft.clear();
            }
            KeyCode::Enter if self.input_mode == InputMode::Filter => {
                self.controller
                    .state_mut()
                    .apply_filter(self.input_draft.clone());
                self.input_mode = InputMode::Normal;
                self.input_draft.clear();
            }
            KeyCode::Enter => {
                self.apply_or_advance_find(key.modifiers.contains(KeyModifiers::SHIFT))
            }
            KeyCode::Backspace => {
                self.input_draft.pop();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER) =>
            {
                self.input_draft.push(character);
            }
            _ => {}
        }
    }

    fn apply_or_advance_find(&mut self, backwards: bool) {
        let result = if self.input_draft != self.controller.state().find_query() {
            self.controller
                .state_mut()
                .set_find(self.input_draft.clone())
        } else if backwards {
            self.controller.state_mut().previous_find()
        } else {
            self.controller.state_mut().next_find()
        };
        self.record(result.map_err(|error| error.to_string()));
    }

    fn toggle_stream(&mut self) {
        let result = if self.controller.is_streaming() {
            self.controller.request_stop_stream()
        } else {
            self.controller.start_stream()
        };
        self.record(result);
    }

    fn refresh(&mut self) {
        let result = if self.overlay != OverlayMode::Devices
            && self.controller.state().tab() == LogTab::FaultLog
        {
            self.fault_scroll = 0;
            self.controller.request_fault_log_refresh()
        } else {
            self.controller.request_device_refresh()
        };
        self.record(result);
    }

    fn move_up(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.previous_fault();
            self.fault_scroll = 0;
        } else {
            self.controller
                .state_mut()
                .scroll_up(rows, self.viewport_height);
        }
    }

    fn move_down(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.next_fault();
            self.fault_scroll = 0;
        } else {
            self.controller
                .state_mut()
                .scroll_down(rows, self.viewport_height);
        }
    }

    fn scroll_fault_down(&mut self, rows: usize) {
        let line_count = self
            .controller
            .selected_fault_entry()
            .map_or(0, |entry| entry.raw.lines().count());
        let max_start = line_count.saturating_sub(self.viewport_height);
        self.fault_scroll = self.fault_scroll.saturating_add(rows).min(max_start);
    }

    fn record(&mut self, result: Result<(), String>) {
        self.action_status.record_action(result);
    }
}

#[cfg(test)]
mod tests {
    use super::RedrawState;

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
}
