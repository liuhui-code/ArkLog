#![cfg(windows)]

use std::fs::{self, File};
use std::io::{self, Read};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CHILD_ENV: &str = "ARKLOG_CONSOLE_TEST_CHILD";
const ENABLE_QUICK_EDIT_MODE: u32 = 0x0040;
const ENABLE_EXTENDED_FLAGS: u32 = 0x0080;
const COOKED_INPUT: u32 = 0x0007;

#[test]
fn native_console_keeps_shortcuts_responsive_and_restores_quick_edit() {
    in_private_console(
        "native_console_keeps_shortcuts_responsive_and_restores_quick_edit",
        exercise_console,
    );
}

#[test]
fn failed_terminal_initialization_restores_console_input_mode() {
    in_private_console(
        "failed_terminal_initialization_restores_console_input_mode",
        exercise_initialization_failure,
    );
}

fn in_private_console(test_name: &str, exercise: fn()) {
    if std::env::var_os(CHILD_ENV).is_some() {
        exercise();
        return;
    }

    // A private console isolates mode changes and injected input from cargo,
    // other tests, and the user's shell. It also works in headless Windows CI.
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", test_name, "--nocapture"])
            .env(CHILD_ENV, "1")
            .creation_flags(0x0000_0010) // CREATE_NEW_CONSOLE
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("private console test"),
    );
    wait_until("console regression must finish without hanging", || {
        child.0.try_wait().expect("test status").is_some()
    });
    let status = child.0.wait().expect("console test status");
    let mut output = String::new();
    child
        .0
        .stdout
        .take()
        .expect("test stdout")
        .read_to_string(&mut output)
        .expect("console test output");
    child
        .0
        .stderr
        .take()
        .expect("test stderr")
        .read_to_string(&mut output)
        .expect("console test errors");
    assert!(status.success(), "{output}");
}

fn exercise_initialization_failure() {
    let input = File::options()
        .read(true)
        .write(true)
        .open("CONIN$")
        .expect("private console input");
    let original = input_mode(&input);
    let trace = std::env::temp_dir().join(format!(
        "arklog-console-init-error-{}.log",
        std::process::id()
    ));
    let mut app = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_arklog"))
            .env("ARKLOG_HDC_PATH", "arklog-test-hdc-does-not-exist")
            .env("ARKLOG_EXECUTION_LOG", &trace)
            .stdin(Stdio::from(input.try_clone().expect("input handle")))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("ArkLog with failing terminal output"),
    );
    // Closing the reader makes EnterAlternateScreen fail after raw mode was
    // enabled, exercising the real binary's initialization error path.
    drop(app.0.stdout.take());
    wait_until("failed terminal initialization must exit", || {
        app.0.try_wait().expect("ArkLog status").is_some()
    });
    assert!(!app.0.wait().expect("ArkLog error exit").success());
    assert_eq!(input_mode(&input), original, "restore input mode on error");
    fs::remove_file(trace).expect("remove execution trace");
}

fn exercise_console() {
    let input = File::options()
        .read(true)
        .write(true)
        .open("CONIN$")
        .expect("private console input");
    let output = File::options()
        .read(true)
        .write(true)
        .open("CONOUT$")
        .expect("private console output");
    let original = input_mode(&input) | ENABLE_EXTENDED_FLAGS | ENABLE_QUICK_EDIT_MODE;
    assert_ne!(
        unsafe { SetConsoleMode(input.as_raw_handle(), original) },
        0
    );
    let trace = std::env::temp_dir().join(format!(
        "arklog-console-regression-{}.log",
        std::process::id()
    ));
    let mut app = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_arklog"))
            .env("ARKLOG_HDC_PATH", "arklog-test-hdc-does-not-exist")
            .env("ARKLOG_EXECUTION_LOG", &trace)
            .stdin(Stdio::from(input.try_clone().expect("input handle")))
            .stdout(Stdio::from(output.try_clone().expect("output handle")))
            .stderr(Stdio::from(output))
            .spawn()
            .expect("ArkLog in native console"),
    );
    wait_until("ArkLog must enter raw mode", || {
        input_mode(&input) & COOKED_INPUT == 0
    });
    send_control_key(&input, b'L');
    wait_until("Clear must finish drawing", || {
        read_trace(&trace).contains("event=clear_frame_finished")
    });

    // Raw mode alone leaves Quick Edit enabled in Crossterm on Windows.
    // A mouse selection can then suspend console I/O and every UI shortcut.
    let running_mode = input_mode(&input);
    assert_eq!(
        running_mode & ENABLE_QUICK_EDIT_MODE,
        0,
        "Quick Edit must be disabled while ArkLog owns the console (mode={running_mode:#x})"
    );
    assert_ne!(running_mode & ENABLE_EXTENDED_FLAGS, 0);

    send_control_key(&input, b'D');
    wait_until("device shortcut must still work after Clear", || {
        read_trace(&trace).contains("name=\"ToggleDevices\"")
    });
    send_control_key(&input, b'Q');
    wait_until("Quit must exit without hanging", || {
        app.0.try_wait().expect("ArkLog status").is_some()
    });
    assert!(app.0.wait().expect("ArkLog exit").success());
    assert_eq!(input_mode(&input), original, "restore shell input mode");
    let trace_text = read_trace(&trace);
    assert!(trace_text.contains("event=app_stop"), "{trace_text}");
    fs::remove_file(trace).expect("remove execution trace");
}

fn input_mode(input: &File) -> u32 {
    let mut mode = 0;
    assert_ne!(
        unsafe { GetConsoleMode(input.as_raw_handle(), &mut mode) },
        0,
        "GetConsoleMode: {}",
        io::Error::last_os_error()
    );
    mode
}

fn send_control_key(input: &File, letter: u8) {
    let mut records = [KeyInputRecord {
        event_type: 1, // KEY_EVENT
        padding: 0,
        key_down: 1,
        repeat_count: 1,
        virtual_key: u16::from(letter),
        scan_code: 0,
        character: u16::from(letter - b'A' + 1),
        control_key_state: 0x0008, // LEFT_CTRL_PRESSED
    }; 2];
    records[1].key_down = 0;
    let mut written = 0;
    assert_ne!(
        unsafe {
            WriteConsoleInputW(
                input.as_raw_handle(),
                records.as_ptr(),
                records.len() as u32,
                &mut written,
            )
        },
        0
    );
    assert_eq!(written, records.len() as u32);
}

fn read_trace(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn wait_until(description: &str, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(Instant::now() < deadline, "{description}");
        thread::sleep(Duration::from_millis(10));
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// INPUT_RECORD's KEY_EVENT_RECORD union starts at byte four and is 16 bytes.
#[derive(Clone, Copy)]
#[repr(C)]
struct KeyInputRecord {
    event_type: u16,
    padding: u16,
    key_down: i32,
    repeat_count: u16,
    virtual_key: u16,
    scan_code: u16,
    character: u16,
    control_key_state: u32,
}

#[link(name = "Kernel32")]
extern "system" {
    fn GetConsoleMode(handle: std::os::windows::io::RawHandle, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: std::os::windows::io::RawHandle, mode: u32) -> i32;
    fn WriteConsoleInputW(
        handle: std::os::windows::io::RawHandle,
        records: *const KeyInputRecord,
        length: u32,
        written: *mut u32,
    ) -> i32;
}
