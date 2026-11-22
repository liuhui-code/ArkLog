use super::TerminalApp;
use arklog::{LogTab, OverlayMode};

impl TerminalApp {
    pub(super) fn toggle_stream(&mut self) {
        let result = self.controller.toggle_stream().map(|_| ());
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record_stream_intent(self.controller.pending_stream_action());
        }
        self.record(result);
    }

    pub(super) fn refresh(&mut self) {
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

    pub(super) fn move_up(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.previous_fault();
            self.fault_scroll = 0;
        } else {
            let result = self
                .controller
                .state_mut()
                .scroll_up(rows, self.viewport_height)
                .map_err(|error| error.to_string());
            self.record(result);
        }
    }

    pub(super) fn move_down(&mut self, rows: u64) {
        if self.controller.state().tab() == LogTab::FaultLog {
            self.controller.next_fault();
            self.fault_scroll = 0;
        } else {
            let result = self
                .controller
                .state_mut()
                .scroll_down(rows, self.viewport_height)
                .map_err(|error| error.to_string());
            self.record(result);
        }
    }

    pub(super) fn scroll_fault_down(&mut self, rows: usize) {
        let line_count = self
            .controller
            .selected_fault_entry()
            .map_or(0, |entry| entry.raw.lines().count());
        let max_start = line_count.saturating_sub(self.viewport_height);
        self.fault_scroll = self.fault_scroll.saturating_add(rows).min(max_start);
    }

    pub(super) fn record(&mut self, result: Result<(), String>) {
        if let (Some(diagnostics), Err(error)) = (&self.diagnostics, &result) {
            diagnostics.record_error("action", error);
        }
        self.action_status.record_action(result);
    }

    pub(super) fn record_background_error(&mut self, error: String) {
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record_error("background", &error);
        }
        self.action_status.record_background(Err(error));
    }

    pub(super) fn observe_runtime(&mut self) {
        if let Some(diagnostics) = &mut self.diagnostics {
            diagnostics.observe(
                self.controller.connection_state(),
                self.controller.stream_state(),
                self.controller.fault_state(),
                self.controller.devices().len(),
            );
        }
    }
}
