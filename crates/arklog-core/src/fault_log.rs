use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceFaultLogStatus {
    Ready,
    Empty,
    Unavailable,
    Unauthorized,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFaultLogRawEntry {
    pub id: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFaultLogFetchResult {
    pub device_id: String,
    pub entries: Vec<DeviceFaultLogRawEntry>,
    pub command: String,
    pub stderr: String,
    pub status: DeviceFaultLogStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFaultLogExportResult {
    pub path: String,
    pub entry_count: u64,
    pub bytes_written: u64,
}

pub struct DeviceFaultLogExporter;

impl DeviceFaultLogExporter {
    pub fn export(
        &self,
        path: &Path,
        device_id: &str,
        entries: &[DeviceFaultLogRawEntry],
    ) -> Result<DeviceFaultLogExportResult, String> {
        if entries.is_empty() {
            return Err("No fault log entries to export".to_string());
        }
        let content = format_export(device_id, entries);
        fs::write(path, content.as_bytes())
            .map_err(|error| format!("Failed to export fault logs: {error}"))?;
        Ok(DeviceFaultLogExportResult {
            path: path.to_string_lossy().into_owned(),
            entry_count: entries.len() as u64,
            bytes_written: content.len() as u64,
        })
    }
}

fn format_export(device_id: &str, entries: &[DeviceFaultLogRawEntry]) -> String {
    let mut content = format!(
        "ArkLog Fault Log Export\nDevice: {device_id}\nEntries: {}\n",
        entries.len()
    );
    for entry in entries {
        content.push_str(&format!("\n===== {} =====\n", entry.id));
        content.push_str(&entry.raw);
        if !entry.raw.ends_with('\n') {
            content.push('\n');
        }
    }
    content
}

pub(crate) fn normalize_fault_log_output(
    device_id: &str,
    command: String,
    stdout: &[u8],
    stderr: &[u8],
    success: bool,
) -> DeviceFaultLogFetchResult {
    let stdout = String::from_utf8_lossy(stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(stderr).replace("\r\n", "\n");
    let entries = if success {
        split_entries(device_id, &stdout)
    } else {
        Vec::new()
    };
    let (status, message) = if contains_ignore_ascii_case(&stdout, "Connect server failed")
        || contains_ignore_ascii_case(&stderr, "Connect server failed")
    {
        (
            DeviceFaultLogStatus::Unavailable,
            first_message_pair(&stdout, &stderr, "Device fault logs unavailable"),
        )
    } else if has_authorization_error(&stdout) || has_authorization_error(&stderr) {
        (
            DeviceFaultLogStatus::Unauthorized,
            first_message_pair(&stdout, &stderr, "Device authorization required"),
        )
    } else if !success {
        (
            DeviceFaultLogStatus::Error,
            first_message(&stderr, "Fault log command failed"),
        )
    } else if entries.is_empty() {
        (
            DeviceFaultLogStatus::Empty,
            "No fault logs found".to_string(),
        )
    } else {
        (DeviceFaultLogStatus::Ready, "ok".to_string())
    };

    DeviceFaultLogFetchResult {
        device_id: device_id.to_string(),
        entries,
        command,
        stderr,
        status,
        message,
    }
}

fn split_entries(device_id: &str, output: &str) -> Vec<DeviceFaultLogRawEntry> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut lines = output.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim().is_empty()
            && !current.trim().is_empty()
            && lines
                .peek()
                .is_some_and(|next| looks_like_entry_start(next.trim()))
        {
            push_entry(device_id, &mut entries, &mut current);
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    push_entry(device_id, &mut entries, &mut current);
    entries
}

fn push_entry(device_id: &str, entries: &mut Vec<DeviceFaultLogRawEntry>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        entries.push(DeviceFaultLogRawEntry {
            id: format!("{device_id}-fault-{}", entries.len() + 1),
            raw: trimmed.to_string(),
        });
    }
    current.clear();
}

fn looks_like_entry_start(line: &str) -> bool {
    matches!(
        line.split(':').next().map(str::trim),
        Some("Timestamp" | "Reason" | "Process" | "PID" | "Summary" | "FaultType")
    )
}

fn first_message(text: &str, fallback: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn first_message_pair(first: &str, second: &str, fallback: &str) -> String {
    let message = first_message(first, "");
    if message.is_empty() {
        first_message(second, fallback)
    } else {
        message
    }
}

fn has_authorization_error(text: &str) -> bool {
    ["unauthorized", "permission denied", "authentication failed"]
        .iter()
        .any(|needle| contains_ignore_ascii_case(text, needle))
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}
