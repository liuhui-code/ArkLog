use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::{configure_hidden_command, spawn_log_reader, LogBatchSink};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogStreamSummary {
    pub stream_id: String,
    pub device_id: String,
    pub status: String,
}

pub struct DeviceLogRuntime {
    executable: String,
    next_stream_id: AtomicU64,
    children: Mutex<HashMap<String, Child>>,
}

impl DeviceLogRuntime {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            next_stream_id: AtomicU64::new(1),
            children: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_stream(
        &self,
        device_id: &str,
        sink: Arc<dyn LogBatchSink>,
    ) -> Result<DeviceLogStreamSummary, String> {
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err("A device id is required".to_string());
        }

        let stream_id = format!(
            "stream-{}",
            self.next_stream_id.fetch_add(1, Ordering::Relaxed)
        );
        let mut command = Command::new(&self.executable);
        configure_hidden_command(&mut command);
        let mut child = command
            .args(["-t", device_id, "hilog"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Failed to start HDC HiLog: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "HDC HiLog stdout was unavailable".to_string())?;

        spawn_log_reader(stream_id.clone(), device_id.to_string(), stdout, sink);
        self.children
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?
            .insert(stream_id.clone(), child);

        Ok(DeviceLogStreamSummary {
            stream_id,
            device_id: device_id.to_string(),
            status: "running".to_string(),
        })
    }

    pub fn stop_stream(&self, stream_id: &str) -> Result<(), String> {
        let mut child = self
            .children
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?
            .remove(stream_id)
            .ok_or_else(|| format!("Unknown device log stream: {stream_id}"))?;
        child
            .kill()
            .map_err(|error| format!("Failed to stop HDC HiLog: {error}"))?;
        child
            .wait()
            .map_err(|error| format!("Failed to reap HDC HiLog: {error}"))?;
        Ok(())
    }
}

impl Drop for DeviceLogRuntime {
    fn drop(&mut self) {
        if let Ok(children) = self.children.get_mut() {
            for child in children.values_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
