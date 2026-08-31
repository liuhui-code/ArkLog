use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use arklog::{
    ConnectionState, ExecutionLog, FaultLogState, RuntimeDiagnostics, StreamAction, StreamState,
};

#[test]
fn writes_structured_execution_events_to_the_requested_file() {
    let path = temporary_log_path();
    let log = ExecutionLog::start(&path, 1024).expect("execution log");

    log.record("command", &[("name", "Ctrl+S"), ("result", "accepted")]);
    drop(log);

    let content = fs::read_to_string(&path).expect("written execution log");
    assert!(content.contains("event=command"), "{content}");
    assert!(content.contains("name=\"Ctrl+S\""), "{content}");
    assert!(content.contains("result=\"accepted\""), "{content}");
    assert!(content.lines().all(|line| line.starts_with("ts_ms=")));
    fs::remove_file(path).expect("remove test log");
}

#[test]
fn restarts_the_execution_log_before_it_exceeds_its_file_budget() {
    let path = temporary_log_path();
    let log = ExecutionLog::start(&path, 192).expect("execution log");

    for sequence in 0..20 {
        let sequence = sequence.to_string();
        log.record("state", &[("sequence", &sequence)]);
    }
    log.record("final_state", &[("stream", "Stopped")]);
    drop(log);

    let content = fs::read_to_string(&path).expect("bounded execution log");
    assert!(content.len() <= 192, "{} bytes", content.len());
    assert!(content.contains("event=final_state"), "{content}");
    fs::remove_file(path).expect("remove test log");
}

#[test]
fn records_commands_state_transitions_and_errors_without_poll_noise() {
    let path = temporary_log_path();
    let log = ExecutionLog::start(&path, 4096).expect("execution log");
    let mut diagnostics = RuntimeDiagnostics::new(
        log,
        &ConnectionState::Ready,
        &StreamState::Streaming,
        &FaultLogState::Idle,
        1,
    );

    diagnostics.record_command("ToggleStream");
    diagnostics.record_stream_intent(Some(StreamAction::Start));
    diagnostics.observe(
        &ConnectionState::Ready,
        &StreamState::Stopping,
        &FaultLogState::Idle,
        1,
    );
    diagnostics.observe(
        &ConnectionState::Ready,
        &StreamState::Stopping,
        &FaultLogState::Idle,
        1,
    );
    diagnostics.record_error("background", "HDC failed\nretry available");
    drop(diagnostics);

    let content = fs::read_to_string(&path).expect("runtime diagnostics");
    assert!(content.contains("event=app_start"), "{content}");
    assert!(
        content.contains("event=command name=\"ToggleStream\""),
        "{content}"
    );
    assert!(content.contains("event=stream_intent pending=\"Start\""));
    assert_eq!(content.matches("stream=\"Stopping\"").count(), 1);
    assert!(content.contains("event=error scope=\"background\""));
    assert!(content.lines().all(|line| line.starts_with("ts_ms=")));
    fs::remove_file(path).expect("remove test log");
}

fn temporary_log_path() -> PathBuf {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "arklog-execution-test-{}-{sequence}.log",
        std::process::id()
    ))
}
