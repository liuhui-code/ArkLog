use std::env;
use std::io;
use std::time::Duration;

use arklog::{
    render_app, run_memory_probe, AppView, ArkLogController, InputMode, LogTab, MEMORY_BUDGET_BYTES,
};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    DefaultTerminal,
};

fn main() -> io::Result<()> {
    if env::args().nth(1).as_deref() == Some("--memory-probe") {
        return run_memory_budget_probe();
    }
    let executable = env::var("ARKLOG_HDC_PATH")
        .or_else(|_| env::var("ARKLOG_HDC"))
        .unwrap_or_else(|_| "hdc".to_string());
    let mut controller = ArkLogController::discover(executable).map_err(io::Error::other)?;
    let startup_error = controller.start_stream().err();
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
        "lines={} visible={} find={} append_ms={} filter_ms={} find_ms={} retained_heap_bytes={} peak_rss_bytes={} budget_bytes={}",
        probe.raw_count,
        probe.visible_count,
        probe.find_count,
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
    input_draft: String,
    action_error: Option<String>,
    should_quit: bool,
    viewport_height: usize,
}

impl TerminalApp {
    fn new(controller: ArkLogController, action_error: Option<String>) -> Self {
        Self {
            controller,
            input_mode: InputMode::Normal,
            input_draft: String::new(),
            action_error,
            should_quit: false,
            viewport_height: 1,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            let pump_result = self.controller.pump_log_batches();
            self.record(pump_result);
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
                let runtime_status = self
                    .action_error
                    .as_deref()
                    .unwrap_or_else(|| self.controller.runtime_status());
                render_app(
                    frame,
                    AppView {
                        device_id: device.map(|device| device.id.as_str()),
                        device_status: device.map_or("No devices", |device| device.status.as_str()),
                        tab: state.tab(),
                        streaming: self.controller.is_streaming(),
                        runtime_status,
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
                        input_draft: &self.input_draft,
                        fault_result: self.controller.fault_result(),
                        selected_fault: self.controller.selected_fault(),
                    },
                );
            })?;

            if event::poll(Duration::from_millis(30))? {
                if let Event::Key(key) = event::read()? {
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        self.handle_key(key);
                    }
                }
            }
        }
        let stop_result = self.controller.stop_stream();
        self.record(stop_result);
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.input_mode != InputMode::Normal {
            self.handle_input_key(key);
            return;
        }
        if key.code == KeyCode::Char('f')
            && key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
        {
            self.input_mode = InputMode::Find;
            self.input_draft = self.controller.state().find_query().to_string();
            return;
        }
        match key.code {
            KeyCode::Char('q' | 'Q') => self.should_quit = true,
            KeyCode::Tab => self.controller.state_mut().next_tab(),
            KeyCode::Char('s' | 'S') => self.toggle_stream(),
            KeyCode::Char('r' | 'R') => self.refresh(),
            KeyCode::Char('c' | 'C') => {
                let result = self
                    .controller
                    .state_mut()
                    .clear()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
            KeyCode::Char('/') if self.controller.state().tab() == LogTab::HiLog => {
                self.input_mode = InputMode::Filter;
                self.input_draft = self.controller.state().filter_query().to_string();
            }
            KeyCode::Char('g' | 'G') | KeyCode::End => {
                self.controller.state_mut().follow_latest();
            }
            KeyCode::Char('n') => {
                let result = self
                    .controller
                    .state_mut()
                    .next_find()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
            KeyCode::Char('N') => {
                let result = self
                    .controller
                    .state_mut()
                    .previous_find()
                    .map_err(|error| error.to_string());
                self.record(result);
            }
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
            KeyCode::PageUp => self.move_up(10),
            KeyCode::PageDown => self.move_down(10),
            _ => {}
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
            self.controller.stop_stream()
        } else {
            self.controller.start_stream()
        };
        self.record(result);
    }

    fn refresh(&mut self) {
        let result = if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.refresh_fault_logs()
        } else {
            self.controller.refresh_devices()
        };
        self.record(result);
    }

    fn move_up(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.previous_fault();
        } else {
            self.controller
                .state_mut()
                .scroll_up(rows, self.viewport_height);
        }
    }

    fn move_down(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.next_fault();
        } else {
            self.controller
                .state_mut()
                .scroll_down(rows, self.viewport_height);
        }
    }

    fn record(&mut self, result: Result<(), String>) {
        self.action_error = result.err();
    }
}
