use std::io::{self, Write};

use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

/// Mouse reporting belongs only to the log body and must not leak into the shell.
pub(crate) struct MouseCapture<W: Write> {
    output: W,
    enabled: bool,
}

impl<W: Write> MouseCapture<W> {
    pub(crate) fn new(output: W) -> Self {
        Self {
            output,
            enabled: false,
        }
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) -> io::Result<()> {
        if self.enabled == enabled {
            return Ok(());
        }
        if enabled {
            // Even a partially written enable sequence must be cleaned up.
            self.enabled = true;
            execute!(self.output, EnableMouseCapture)
        } else {
            execute!(self.output, DisableMouseCapture)?;
            self.enabled = false;
            Ok(())
        }
    }
}

impl<W: Write> Drop for MouseCapture<W> {
    fn drop(&mut self) {
        let _ = self.set_enabled(false);
    }
}
