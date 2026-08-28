use std::process::Command;

use serde::Serialize;

mod runtime;
mod stream;

pub use runtime::{DeviceLogRuntime, DeviceLogStreamSummary};
pub use stream::{spawn_log_reader, DeviceLogOutputBatch, LogBatchSink};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogDevice {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

pub struct CommandOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait CommandRunner: Send + Sync {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String>;
}

pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        let mut command = Command::new(program);
        configure_hidden_command(&mut command);
        let output = command
            .args(args)
            .output()
            .map_err(|error| format!("Failed to run {program}: {error}"))?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

pub struct HdcClient<R = SystemCommandRunner> {
    executable: String,
    runner: R,
}

impl HdcClient<SystemCommandRunner> {
    pub fn new(executable: impl Into<String>) -> Self {
        Self::with_runner(executable, SystemCommandRunner)
    }
}

impl<R: CommandRunner> HdcClient<R> {
    pub fn with_runner(executable: impl Into<String>, runner: R) -> Self {
        Self {
            executable: executable.into(),
            runner,
        }
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceLogDevice>, String> {
        let args = ["list", "targets", "-v"].map(str::to_string);
        let output = self.runner.output(&self.executable, &args)?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        let devices = parse_hdc_targets(&combined);
        if !output.success && devices.is_empty() {
            return Err(combined.trim().to_string());
        }
        Ok(devices)
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
            let status = match parts
                .next()
                .unwrap_or("unknown")
                .to_ascii_lowercase()
                .as_str()
            {
                "connected" | "online" => "online",
                "offline" => "offline",
                "unauthorized" => "unauthorized",
                _ => "unknown",
            };
            Some(DeviceLogDevice {
                label: id.clone(),
                id,
                status: status.to_string(),
                detail: detail.to_string(),
            })
        })
        .collect()
}

fn configure_hidden_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}
