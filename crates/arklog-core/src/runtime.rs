use std::collections::HashMap;
use std::fmt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde::Serialize;

use crate::{configure_hidden_command, spawn_log_reader, terminate_process_tree, LogBatchSink};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogStreamSummary {
    pub stream_id: String,
    pub device_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogStreamExit {
    pub stream_id: String,
    pub success: bool,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLogStartMode {
    ExistingBuffer,
    LiveOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceLogStartError {
    UnsupportedLiveStart { reason: &'static str },
    Launch(String),
}

impl fmt::Display for DeviceLogStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLiveStart { reason } => {
                write!(formatter, "UnsupportedLiveStart: {reason}")
            }
            Self::Launch(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DeviceLogStartError {}

pub struct DeviceLogRuntime {
    executable: String,
    next_stream_id: AtomicU64,
    streams: Mutex<HashMap<String, ActiveStream>>,
}

struct ActiveStream {
    child: Child,
    worker: JoinHandle<()>,
}

impl DeviceLogRuntime {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            next_stream_id: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
        }
    }

    pub fn start_stream(
        &self,
        device_id: &str,
        sink: Arc<dyn LogBatchSink>,
    ) -> Result<DeviceLogStreamSummary, String> {
        self.start_stream_with_mode(device_id, DeviceLogStartMode::ExistingBuffer, sink)
            .map_err(|error| error.to_string())
    }

    pub fn start_stream_with_mode(
        &self,
        device_id: &str,
        mode: DeviceLogStartMode,
        sink: Arc<dyn LogBatchSink>,
    ) -> Result<DeviceLogStreamSummary, DeviceLogStartError> {
        if mode == DeviceLogStartMode::LiveOnly {
            return Err(DeviceLogStartError::UnsupportedLiveStart {
                reason: "the supported HDC/HiLog command surface has no verified server-side cursor or attach boundary",
            });
        }
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err(DeviceLogStartError::Launch(
                "A device id is required".to_string(),
            ));
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
            .map_err(|error| {
                DeviceLogStartError::Launch(format!("Failed to start HDC HiLog: {error}"))
            })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            DeviceLogStartError::Launch("HDC HiLog stdout was unavailable".to_string())
        })?;

        let worker = spawn_log_reader(stream_id.clone(), device_id.to_string(), stdout, sink);
        self.streams
            .lock()
            .map_err(|_| {
                DeviceLogStartError::Launch("Device log runtime lock was poisoned".to_string())
            })?
            .insert(stream_id.clone(), ActiveStream { child, worker });

        Ok(DeviceLogStreamSummary {
            stream_id,
            device_id: device_id.to_string(),
            status: "running".to_string(),
        })
    }

    pub fn stop_stream(&self, stream_id: &str) -> Result<(), String> {
        let mut stream = self
            .streams
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?
            .remove(stream_id)
            .ok_or_else(|| format!("Unknown device log stream: {stream_id}"))?;
        if let Err(error) = terminate_process_tree(&mut stream.child) {
            self.restore_stream(stream_id, stream)?;
            return Err(format!("Failed to stop HDC HiLog: {error}"));
        }
        if let Err(error) = stream.child.wait() {
            self.restore_stream(stream_id, stream)?;
            return Err(format!("Failed to reap HDC HiLog: {error}"));
        }
        stream
            .worker
            .join()
            .map_err(|_| "Device log worker panicked while stopping".to_string())?;
        Ok(())
    }

    pub fn has_stream(&self, stream_id: &str) -> bool {
        self.streams
            .lock()
            .is_ok_and(|streams| streams.contains_key(stream_id))
    }

    pub fn reap_finished_stream(
        &self,
        stream_id: &str,
    ) -> Result<Option<DeviceLogStreamExit>, String> {
        let Some(exit) = self.finished_stream_status(stream_id)? else {
            return Ok(None);
        };
        let mut streams = self
            .streams
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?;
        let stream = streams
            .remove(stream_id)
            .ok_or_else(|| format!("Unknown device log stream: {stream_id}"))?;
        drop(streams);
        stream
            .worker
            .join()
            .map_err(|_| "Device log worker panicked while reaping".to_string())?;
        Ok(Some(exit))
    }

    pub fn finished_stream_status(
        &self,
        stream_id: &str,
    ) -> Result<Option<DeviceLogStreamExit>, String> {
        let mut streams = self
            .streams
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?;
        let status = streams
            .get_mut(stream_id)
            .ok_or_else(|| format!("Unknown device log stream: {stream_id}"))?
            .child
            .try_wait()
            .map_err(|error| format!("Failed to inspect HDC HiLog: {error}"))?;
        Ok(status.map(|status| DeviceLogStreamExit {
            stream_id: stream_id.to_string(),
            success: status.success(),
            code: status.code(),
        }))
    }

    fn restore_stream(&self, stream_id: &str, stream: ActiveStream) -> Result<(), String> {
        self.streams
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?
            .insert(stream_id.to_string(), stream);
        Ok(())
    }
}

impl Drop for DeviceLogRuntime {
    fn drop(&mut self) {
        if let Ok(streams) = self.streams.get_mut() {
            for (_, mut stream) in streams.drain() {
                let _ = terminate_process_tree(&mut stream.child);
                let _ = stream.child.wait();
                let _ = stream.worker.join();
            }
        }
    }
}
