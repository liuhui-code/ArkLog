use std::io::{BufRead, BufReader, Read};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use serde::Serialize;

const MAX_BATCH_LINES: usize = 50;
const MAX_PENDING_LINES: usize = 256;
const FLUSH_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogOutputBatch {
    pub stream_id: String,
    pub device_id: String,
    pub lines: Vec<String>,
}

pub trait LogBatchSink: Send + Sync + 'static {
    fn deliver(&self, batch: DeviceLogOutputBatch);
}

pub fn spawn_log_reader<R>(
    stream_id: String,
    device_id: String,
    reader: R,
    sink: Arc<dyn LogBatchSink>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(MAX_PENDING_LINES);
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            match reader.read_until(b'\n', &mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = decode_log_line(&bytes);
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });

    thread::spawn(move || {
        let mut lines = Vec::with_capacity(MAX_BATCH_LINES);
        loop {
            match receiver.recv_timeout(FLUSH_INTERVAL) {
                Ok(line) => {
                    lines.push(line);
                    if lines.len() >= MAX_BATCH_LINES {
                        deliver(&stream_id, &device_id, &sink, &mut lines);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    deliver(&stream_id, &device_id, &sink, &mut lines);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    deliver(&stream_id, &device_id, &sink, &mut lines);
                    break;
                }
            }
        }
    })
}

fn deliver(
    stream_id: &str,
    device_id: &str,
    sink: &Arc<dyn LogBatchSink>,
    lines: &mut Vec<String>,
) {
    if lines.is_empty() {
        return;
    }
    sink.deliver(DeviceLogOutputBatch {
        stream_id: stream_id.to_string(),
        device_id: device_id.to_string(),
        lines: std::mem::take(lines),
    });
}

fn decode_log_line(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).to_string()
}
