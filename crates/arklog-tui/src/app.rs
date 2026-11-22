use crate::{mouse_capture::MouseCapture, system_clipboard::SystemClipboard};
use arklog::{
    app_layout, render_app, ActionStatus, AppView, ArkLogController, ExecutionLog, InputMode,
    LogTab, McpRuntime, OverlayMode, RuntimeDiagnostics, TextInput,
};
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind, MouseEvent},
    layout::Rect,
    DefaultTerminal,
};
use std::{
    io,
    time::{Duration, Instant},
};

mod actions;
mod history;
mod input;
pub(crate) use history::history_path;
const MAX_LOG_BATCHES_PER_TICK: usize = 8;
#[cfg(test)]
mod suggestion_tests;
#[cfg(test)]
mod tests;

pub(crate) struct TerminalApp {
    controller: ArkLogController,
    input_mode: InputMode,
    overlay: OverlayMode,
    input: TextInput,
    clipboard: SystemClipboard,
    action_status: ActionStatus,
    should_quit: bool,
    viewport_height: usize,
    fault_scroll: usize,
    diagnostics: Option<RuntimeDiagnostics>,
    trace_clear_frame: bool,
    history_path: Option<std::path::PathBuf>,
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
    pub(crate) fn new(
        mut controller: ArkLogController,
        action_error: Option<String>,
        execution_log: Option<ExecutionLog>,
    ) -> Self {
        let diagnostics = execution_log.map(|log| {
            RuntimeDiagnostics::new(
                log,
                controller.connection_state(),
                controller.stream_state(),
                controller.fault_state(),
                controller.devices().len(),
            )
        });
        if let (Some(diagnostics), Some(error)) = (&diagnostics, &action_error) {
            diagnostics.record_error("startup", error);
        }
        controller.request_stream_running();
        Self {
            controller,
            input_mode: InputMode::Normal,
            overlay: OverlayMode::None,
            input: TextInput::new(""),
            clipboard: SystemClipboard::default(),
            action_status: ActionStatus::new(action_error),
            should_quit: false,
            viewport_height: 1,
            fault_scroll: 0,
            diagnostics,
            trace_clear_frame: false,
            history_path: None,
        }
    }

    pub(crate) fn run(
        mut self,
        terminal: &mut DefaultTerminal,
        mcp: Option<&McpRuntime>,
    ) -> io::Result<()> {
        let mut mouse_capture = MouseCapture::new(io::stdout());
        let mut redraw = RedrawState::new();
        while !self.should_quit {
            mouse_capture.set_enabled(self.browsing_hilog())?;
            match self
                .controller
                .pump_log_batches_limited(MAX_LOG_BATCHES_PER_TICK)
            {
                Ok(batch_count) => redraw.mark_if(batch_count > 0),
                Err(error) => {
                    self.record_background_error(error);
                    redraw.mark();
                }
            }
            match self.controller.pump_background_tasks_with_activity() {
                Ok(changed) => redraw.mark_if(changed),
                Err(error) => {
                    self.record_background_error(error);
                    redraw.mark();
                }
            }
            if let Some(mcp) = mcp {
                mcp.pump(&mut self.controller);
            }
            self.observe_runtime();
            if redraw.take() {
                let trace_clear_frame = std::mem::take(&mut self.trace_clear_frame);
                let frame_started = Instant::now();
                if trace_clear_frame {
                    if let Some(diagnostics) = &self.diagnostics {
                        diagnostics.record_clear_frame_started();
                    }
                }
                let size = terminal.size()?;
                let geometry = app_layout(size.into());
                self.viewport_height = geometry.log_content_height;
                self.controller
                    .state_mut()
                    .set_viewport_size(geometry.log_content_width, geometry.log_content_height);
                let window = self.controller.state_mut().display_window()?;
                let window_start = window.start_record;
                let window_start_cell = window.start_cell;
                let lines = window.records;
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
                            stream_state: self.controller.stream_state(),
                            pending_stream_action: self.controller.pending_stream_action(),
                            connection_status: self.controller.connection_state().message(),
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
                            window_start_cell,
                            horizontal_offset: state.horizontal_offset(),
                            soft_wrap: state.soft_wrap(),
                            lines: &lines,
                            log_selection: state.log_selection(),
                            input_mode: self.input_mode,
                            overlay: self.overlay,
                            input: self.input_view(),
                            fault_result: self.controller.fault_result(),
                            selected_fault: self.controller.selected_fault(),
                            fault_scroll: self.fault_scroll,
                        },
                    );
                })?;
                if trace_clear_frame {
                    if let Some(diagnostics) = &self.diagnostics {
                        diagnostics.record_clear_frame_finished(frame_started.elapsed());
                    }
                }
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
                    Event::Mouse(mouse) => {
                        redraw.mark_if(self.handle_mouse(mouse, terminal.size()?.into()));
                    }
                    Event::Paste(text) if self.input_mode != InputMode::Normal => {
                        self.input.insert_text(&text);
                        redraw.mark();
                    }
                    _ => {}
                }
            }
        }
        let stop_result = self.controller.stop_stream();
        self.record(stop_result);
        self.observe_runtime();
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record_app_stop();
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) -> bool {
        if !self.browsing_hilog() {
            return false;
        }
        let content = app_layout(area).log_content;
        match self.controller.state_mut().handle_log_mouse(mouse, content) {
            Ok(changed) => changed,
            Err(error) => {
                self.record(Err(error.to_string()));
                true
            }
        }
    }

    fn browsing_hilog(&self) -> bool {
        self.input_mode == InputMode::Normal
            && self.overlay == OverlayMode::None
            && self.controller.state().tab() == LogTab::HiLog
    }
}
