use std::env;
use std::io;
use std::path::PathBuf;

use arklog::{
    run_memory_probe, serve_mcp, ArkLogController, ExecutionLog, McpRuntime, DEFAULT_MCP_PORT,
    EXECUTION_LOG_MAX_BYTES, MEMORY_BUDGET_BYTES,
};
mod app;
use app::TerminalApp;

mod mouse_capture;
mod system_clipboard;
#[cfg(windows)]
mod windows_console;

#[cfg(windows)]
const WINDOWS_TERMINAL_CHILD_ARG: &str = "--windows-terminal-child";
#[cfg(windows)]
const WINDOWS_TERMINAL_ARG: &str = "--windows-terminal";

fn main() -> io::Result<()> {
    if env::args().nth(1).as_deref() == Some("--mcp") {
        let port = configured_mcp_port()?.unwrap_or(DEFAULT_MCP_PORT);
        return serve_mcp(io::stdin().lock(), io::stdout().lock(), port);
    }
    if env::args().nth(1).as_deref() == Some("--memory-probe") {
        return run_memory_budget_probe();
    }
    if relaunch_standalone_windows_console() {
        return Ok(());
    }
    let mcp_runtime = configured_mcp_port()?.map(McpRuntime::start).transpose()?;
    let executable = env::var("ARKLOG_HDC_PATH")
        .or_else(|_| env::var("ARKLOG_HDC"))
        .unwrap_or_else(|_| "hdc".to_string());
    let execution_log_path = env::var_os("ARKLOG_EXECUTION_LOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir().join("arklog-execution.log"));
    let (execution_log, diagnostic_error) =
        match ExecutionLog::start(&execution_log_path, EXECUTION_LOG_MAX_BYTES) {
            Ok(log) => (Some(log), None),
            Err(error) => (None, Some(format!("Execution log unavailable: {error}"))),
        };
    let execution_log_enabled = execution_log.is_some();
    let mut controller = ArkLogController::new(executable, Vec::new()).map_err(io::Error::other)?;
    let startup_error = [controller.request_device_refresh().err(), diagnostic_error]
        .into_iter()
        .flatten()
        .reduce(|left, right| format!("{left}; {right}"));
    let mut app = TerminalApp::new(controller, startup_error, execution_log);
    app.restore_history(app::history_path());
    #[cfg(windows)]
    let _console_mode = windows_console::ConsoleModeGuard::new()?;
    let mut terminal = ratatui::try_init()?;
    let result = app.run(&mut terminal, mcp_runtime.as_ref());
    ratatui::try_restore()?;
    if execution_log_enabled {
        eprintln!("ArkLog execution log: {}", execution_log_path.display());
    }
    result
}

fn configured_mcp_port() -> io::Result<Option<u16>> {
    env::var("ARKLOG_MCP_PORT")
        .ok()
        .map(|port| {
            let port = port
                .parse::<u16>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
            if port == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "ARKLOG_MCP_PORT must be between 1 and 65535",
                ));
            }
            Ok(port)
        })
        .transpose()
}

#[cfg(any(windows, test))]
fn should_relaunch_in_windows_terminal(
    explicitly_requested: bool,
    is_windows: bool,
    already_in_windows_terminal: bool,
    console_process_count: u32,
) -> bool {
    explicitly_requested && is_windows && !already_in_windows_terminal && console_process_count == 1
}

#[cfg(windows)]
fn relaunch_standalone_windows_console() -> bool {
    let explicitly_requested = env::args_os().any(|arg| arg == WINDOWS_TERMINAL_ARG);
    let already_in_windows_terminal = env::var_os("WT_SESSION").is_some()
        || env::args_os().any(|arg| arg == WINDOWS_TERMINAL_CHILD_ARG);
    if !should_relaunch_in_windows_terminal(
        explicitly_requested,
        true,
        already_in_windows_terminal,
        windows_console_process_count(),
    ) {
        return false;
    }
    let Ok(current_executable) = env::current_exe() else {
        return false;
    };
    std::process::Command::new("wt.exe")
        .arg("new-tab")
        .arg(current_executable)
        .arg(WINDOWS_TERMINAL_CHILD_ARG)
        .args(
            env::args_os()
                .skip(1)
                .filter(|arg| arg != WINDOWS_TERMINAL_ARG),
        )
        .spawn()
        .is_ok()
}

#[cfg(windows)]
fn windows_console_process_count() -> u32 {
    let mut process_ids = [0_u32; 2];
    unsafe { GetConsoleProcessList(process_ids.as_mut_ptr(), process_ids.len() as u32) }
}

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn GetConsoleProcessList(process_list: *mut u32, process_count: u32) -> u32;
}

#[cfg(not(windows))]
fn relaunch_standalone_windows_console() -> bool {
    false
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
