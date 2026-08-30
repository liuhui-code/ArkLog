use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use arklog_core::{
    CommandOutput, CommandRunner, DeviceFaultLogExporter, DeviceFaultLogRawEntry,
    DeviceFaultLogStatus, HdcClient,
};

#[test]
fn fetches_and_splits_device_fault_logs_through_hdc() {
    let client = HdcClient::with_runner("hdc", ReadyFaultLogRunner);

    let result = client.list_fault_logs("USB-01").expect("fault logs");

    assert_eq!(result.device_id, "USB-01");
    assert_eq!(result.status, DeviceFaultLogStatus::Ready);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.entries[0].id, "USB-01-fault-1");
    assert!(result.entries[0].raw.contains("at render"));
    assert!(result.entries[1].raw.contains("APP_KILLED"));
    assert_eq!(result.command, "hdc -t USB-01 shell faultloggerd --dump");
}

#[test]
fn reports_connect_server_failure_as_unavailable_data() {
    let client = HdcClient::with_runner("hdc", UnavailableFaultLogRunner);

    let result = client.list_fault_logs("USB-01").expect("structured result");

    assert_eq!(result.status, DeviceFaultLogStatus::Unavailable);
    assert!(result.entries.is_empty());
    assert_eq!(result.message, "Connect server failed");
}

#[test]
fn reports_permission_failure_as_unauthorized_data() {
    let client = HdcClient::with_runner("hdc", UnauthorizedFaultLogRunner);

    let result = client.list_fault_logs("USB-01").expect("structured result");

    assert_eq!(result.status, DeviceFaultLogStatus::Unauthorized);
    assert_eq!(result.message, "permission denied: device unauthorized");
}

#[test]
fn reports_empty_and_generic_command_results_without_entries() {
    let empty = HdcClient::with_runner(
        "hdc",
        FixedFaultLogRunner {
            success: true,
            stderr: "",
        },
    )
    .list_fault_logs("USB-01")
    .expect("empty result");
    let failed = HdcClient::with_runner(
        "hdc",
        FixedFaultLogRunner {
            success: false,
            stderr: "faultloggerd failed to dump",
        },
    )
    .list_fault_logs("USB-01")
    .expect("error result");

    assert_eq!(empty.status, DeviceFaultLogStatus::Empty);
    assert_eq!(empty.message, "No fault logs found");
    assert_eq!(failed.status, DeviceFaultLogStatus::Error);
    assert_eq!(failed.message, "faultloggerd failed to dump");
    assert!(failed.entries.is_empty());
}

#[test]
fn exports_ordered_raw_fault_entries_to_the_selected_text_file() {
    let root = std::env::temp_dir().join(format!(
        "arklog-fault-export-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("temp directory");
    let path = root.join("faults.txt");
    fs::write(&path, "old content").expect("existing file");
    let entries = vec![
        DeviceFaultLogRawEntry {
            id: "fault-2".to_string(),
            raw: "Reason: APP_FREEZE\nSummary: blocked".to_string(),
        },
        DeviceFaultLogRawEntry {
            id: "fault-1".to_string(),
            raw: "Reason: JS_ERROR\n\nStacktrace:\n  at render".to_string(),
        },
    ];

    let result = DeviceFaultLogExporter
        .export(&path, "USB-01", &entries)
        .expect("export");
    let content = fs::read_to_string(&path).expect("exported content");

    assert_eq!(result.entry_count, 2);
    assert_eq!(result.bytes_written, content.len() as u64);
    assert_eq!(result.path, path.to_string_lossy());
    assert_eq!(
        content,
        "ArkLog Fault Log Export\nDevice: USB-01\nEntries: 2\n\n===== fault-2 =====\nReason: APP_FREEZE\nSummary: blocked\n\n===== fault-1 =====\nReason: JS_ERROR\n\nStacktrace:\n  at render\n"
    );

    fs::remove_dir_all(root).expect("cleanup");
}

struct ReadyFaultLogRunner;

struct UnavailableFaultLogRunner;

struct UnauthorizedFaultLogRunner;

struct FixedFaultLogRunner {
    success: bool,
    stderr: &'static str,
}

impl CommandRunner for ReadyFaultLogRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        assert_eq!(program, "hdc");
        assert_eq!(args, ["-t", "USB-01", "shell", "faultloggerd", "--dump"]);
        Ok(CommandOutput {
            success: true,
            stdout: b"Reason: JS_ERROR\nSummary: Render failed\n\nStacktrace:\n  at render (index.ets:4:2)\n\nReason: APP_KILLED\nSummary: Force stop\n".to_vec(),
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for UnavailableFaultLogRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: false,
            stdout: Vec::new(),
            stderr: b"Connect server failed\n".to_vec(),
        })
    }
}

impl CommandRunner for UnauthorizedFaultLogRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: false,
            stdout: Vec::new(),
            stderr: b"permission denied: device unauthorized\n".to_vec(),
        })
    }
}

impl CommandRunner for FixedFaultLogRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: self.success,
            stdout: Vec::new(),
            stderr: self.stderr.as_bytes().to_vec(),
        })
    }
}
