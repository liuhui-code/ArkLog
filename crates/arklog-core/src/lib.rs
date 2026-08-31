use std::time::Duration;

use serde::Serialize;

mod command;
mod fault_log;
mod runtime;
mod stream;

pub use fault_log::{
    DeviceFaultLogExportResult, DeviceFaultLogExporter, DeviceFaultLogFetchResult,
    DeviceFaultLogRawEntry, DeviceFaultLogStatus,
};
pub use runtime::{DeviceLogRuntime, DeviceLogStreamExit, DeviceLogStreamSummary};
pub use stream::{spawn_log_reader, DeviceLogOutputBatch, LogBatchSink};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogDevice {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

pub struct HdcClient<R = SystemCommandRunner> {
    executable: String,
    runner: R,
    device_policy: CommandPolicy,
    fault_policy: CommandPolicy,
}

impl HdcClient<SystemCommandRunner> {
    pub fn new(executable: impl Into<String>) -> Self {
        Self::with_runner(executable, SystemCommandRunner)
    }
}

impl<R: CommandRunner> HdcClient<R> {
    pub fn with_runner(executable: impl Into<String>, runner: R) -> Self {
        Self::with_policies(
            executable,
            runner,
            CommandPolicy::new(Duration::from_secs(3), 256 * 1024),
            CommandPolicy::new(Duration::from_secs(5), 4 * 1024 * 1024),
        )
    }

    pub fn with_policies(
        executable: impl Into<String>,
        runner: R,
        device_policy: CommandPolicy,
        fault_policy: CommandPolicy,
    ) -> Self {
        Self {
            executable: executable.into(),
            runner,
            device_policy,
            fault_policy,
        }
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceLogDevice>, String> {
        let args = ["list", "targets", "-v"].map(str::to_string);
        let output = self
            .runner
            .output_with_policy(&self.executable, &args, self.device_policy)?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        let devices = parse_hdc_targets(&combined);
        if !output.success && devices.is_empty() {
            let message = combined.trim();
            return Err(if message.is_empty() {
                "HDC device discovery failed".to_string()
            } else {
                message.to_string()
            });
        }
        Ok(devices)
    }

    pub fn list_fault_logs(&self, device_id: &str) -> Result<DeviceFaultLogFetchResult, String> {
        if device_id.trim().is_empty() {
            return Err("Device id is required".to_string());
        }
        let args = [
            "-t",
            device_id,
            "shell",
            "hidumper -s 1201 -a \"-p Faultlogger -l -d\"",
        ]
        .map(str::to_string);
        let command = format!("{} {}", self.executable, args.join(" "));
        let output = self
            .runner
            .output_with_policy(&self.executable, &args, self.fault_policy)?;
        Ok(fault_log::normalize_fault_log_output(
            device_id,
            command,
            &output.stdout,
            &output.stderr,
            output.success,
        ))
    }
}

fn parse_hdc_targets(output: &str) -> Vec<DeviceLogDevice> {
    output
        .lines()
        .filter_map(|line| {
            let detail = line.trim();
            if detail.is_empty() || detail.contains("Connect server failed") {
                return None;
            }
            let mut parts = detail.split_whitespace();
            let id = parts.next()?.to_string();
            let status = parts.find_map(normalize_device_status)?;
            Some(DeviceLogDevice {
                label: id.clone(),
                id,
                status: status.to_string(),
                detail: detail.to_string(),
            })
        })
        .collect()
}

fn normalize_device_status(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "ready" | "connected" | "online" => Some("online"),
        "offline" => Some("offline"),
        "unauthorized" => Some("unauthorized"),
        "unknown" => Some("unknown"),
        _ => None,
    }
}

pub(crate) use command::{configure_hidden_command, terminate_process_tree};
pub use command::{CommandOutput, CommandPolicy, CommandRunner, SystemCommandRunner};
