use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

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

        let worker = spawn_log_reader(stream_id.clone(), device_id.to_string(), stdout, sink);
        self.streams
            .lock()
            .map_err(|_| "Device log runtime lock was poisoned".to_string())?
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
        stream
            .child
            .kill()
            .map_err(|error| format!("Failed to stop HDC HiLog: {error}"))?;
        stream
            .child
            .wait()
            .map_err(|error| format!("Failed to reap HDC HiLog: {error}"))?;
        stream
            .worker
            .join()
            .map_err(|_| "Device log worker panicked while stopping".to_string())?;
        Ok(())
    }
}

impl Drop for DeviceLogRuntime {
    fn drop(&mut self) {
        if let Ok(streams) = self.streams.get_mut() {
            for (_, mut stream) in streams.drain() {
                let _ = stream.child.kill();
                let _ = stream.child.wait();
                let _ = stream.worker.join();
            }
        }
    }
}
