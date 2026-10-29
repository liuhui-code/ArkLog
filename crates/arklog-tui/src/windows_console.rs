use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, RawHandle};

const ENABLE_QUICK_EDIT_MODE: u32 = 0x0040;
const ENABLE_EXTENDED_FLAGS: u32 = 0x0080;

/// Owns the console input mode for the terminal session, including error exits.
pub(super) struct ConsoleModeGuard {
    input: File,
    original_mode: u32,
}

impl ConsoleModeGuard {
    pub(super) fn new() -> io::Result<Self> {
        // Match Crossterm's console input even when stdin is redirected.
        let input = File::options().read(true).write(true).open("CONIN$")?;
        let mut original_mode = 0;
        if unsafe { GetConsoleMode(input.as_raw_handle(), &mut original_mode) } == 0 {
            return Err(io::Error::last_os_error());
        }

        // Crossterm's raw mode preserves Quick Edit. Selecting console text
        // would then suspend console I/O on the UI thread, including Quit.
        // EXTENDED_FLAGS is required for Windows to apply a Quick Edit change.
        let mode = (original_mode | ENABLE_EXTENDED_FLAGS) & !ENABLE_QUICK_EDIT_MODE;
        if unsafe { SetConsoleMode(input.as_raw_handle(), mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            input,
            original_mode,
        })
    }
}

impl Drop for ConsoleModeGuard {
    fn drop(&mut self) {
        // Run after Ratatui restores raw mode, so the shell gets its exact
        // previous settings rather than Crossterm's cooked-mode defaults.
        unsafe {
            SetConsoleMode(self.input.as_raw_handle(), self.original_mode);
        }
    }
}

#[link(name = "Kernel32")]
extern "system" {
    fn GetConsoleMode(handle: RawHandle, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: RawHandle, mode: u32) -> i32;
}
