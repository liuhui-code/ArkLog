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
pub use runtime::{
    DeviceLogRuntime, DeviceLogStartError, DeviceLogStartMode, DeviceLogStreamExit,
    DeviceLogStreamSummary,
};
pub use stream::{spawn_log_reader, DeviceLogOutputBatch, LogBatchSink};

const PHONE_CODE_COMMAND: &str = "ls -1 /version/special_cust";
const PHONE_CODE_LOOKUP_TIMEOUT: Duration = Duration::from_secs(1);
const PHONE_CODE_OUTPUT_LIMIT_BYTES: usize = 4 * 1024;
const PHONE_CODE_MAX_CHARS: usize = 128;
const DEVICE_LABEL_SEPARATOR: &str = " ";

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
        let mut devices = parse_hdc_targets(&combined);
        if !output.success && devices.is_empty() {
            let message = combined.trim();
            return Err(if message.is_empty() {
                "HDC device discovery failed".to_string()
            } else {
                message.to_string()
            });
        }
        for device in devices
            .iter_mut()
            .filter(|device| device.status == "online")
        {
            if let Some(phone_code) = self.phone_code(&device.id) {
                device.label = format!("{phone_code}{DEVICE_LABEL_SEPARATOR}{}", device.id);
            }
        }
        Ok(devices)
    }

    fn phone_code(&self, device_id: &str) -> Option<String> {
        let args = ["-t", device_id, "shell", PHONE_CODE_COMMAND].map(str::to_string);
        let output = self
            .runner
            .output_with_policy(
                &self.executable,
                &args,
                CommandPolicy::new(PHONE_CODE_LOOKUP_TIMEOUT, PHONE_CODE_OUTPUT_LIMIT_BYTES),
            )
            .ok()?;
        output
            .success
            .then(|| parse_phone_code(&output.stdout))
            .flatten()
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
            let line = line.trim();
            if line.is_empty() || line.contains("Connect server failed") {
                return None;
            }
            let fields = line.split_whitespace().collect::<Vec<_>>();
            let id = fields.first()?.to_string();
            let (status, detail) = match fields.as_slice() {
                [_, status, detail @ ..] => {
                    normalize_device_status(status).map(|status| (status, detail.join(" ")))
                }
                _ => None,
            }
            .or_else(|| match fields.as_slice() {
                [_, transport, status, detail @ ..] if is_hdc_transport(transport) => {
                    normalize_device_status(status).map(|status| {
                        let detail = std::iter::once(*transport)
                            .chain(detail.iter().copied())
                            .collect::<Vec<_>>()
                            .join(" ");
                        (status, detail)
                    })
                }
                _ => None,
            })?;
            Some(DeviceLogDevice {
                label: id.clone(),
                id,
                status: status.to_string(),
                detail,
            })
        })
        .collect()
}

fn is_hdc_transport(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "usb" | "tcp")
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

fn parse_phone_code(output: &[u8]) -> Option<String> {
    let output = std::str::from_utf8(output).ok()?;
    output.lines().find_map(|line| {
        let candidate = line.trim();
        (!candidate.is_empty()
            && candidate.chars().count() <= PHONE_CODE_MAX_CHARS
            && !candidate.chars().any(char::is_whitespace)
            && !candidate.contains(['/', '\\']))
        .then(|| candidate.to_string())
    })
}

pub(crate) use command::{configure_hidden_command, terminate_process_tree};
pub use command::{CommandOutput, CommandPolicy, CommandRunner, SystemCommandRunner};
