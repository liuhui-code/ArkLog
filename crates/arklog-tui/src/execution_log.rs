use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{ConnectionState, FaultLogState, StreamAction, StreamState};

const EVENT_QUEUE_CAPACITY: usize = 128;
pub const EXECUTION_LOG_MAX_BYTES: usize = 1024 * 1024;

pub struct ExecutionLog {
    path: PathBuf,
    sender: Option<SyncSender<String>>,
    worker: Option<JoinHandle<()>>,
}

impl ExecutionLog {
    pub fn start(path: impl AsRef<Path>, max_bytes: usize) -> io::Result<Self> {
        if max_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "execution log limit must be greater than zero",
            ));
        }
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&path)?;
        file.seek(SeekFrom::End(0))?;
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let worker = thread::spawn(move || write_events(file, receiver, max_bytes));
        Ok(Self {
            path,
            sender: Some(sender),
            worker: Some(worker),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record(&self, event: &str, fields: &[(&str, &str)]) {
        let mut line = format!("ts_ms={} event={}", unix_time_millis(), token(event));
        for (key, value) in fields {
            line.push(' ');
            line.push_str(&token(key));
            line.push_str("=\"");
            line.push_str(&quoted(value));
            line.push('"');
        }
        line.push('\n');
        if let Some(sender) = &self.sender {
            match sender.try_send(line) {
                Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
            }
        }
    }
}

impl Drop for ExecutionLog {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeSnapshot {
    connection: ConnectionState,
    stream: StreamState,
    fault: FaultLogState,
    device_count: usize,
}

pub struct RuntimeDiagnostics {
    log: ExecutionLog,
    last: Option<RuntimeSnapshot>,
}

impl RuntimeDiagnostics {
    pub fn new(
        log: ExecutionLog,
        connection: &ConnectionState,
        stream: &StreamState,
        fault: &FaultLogState,
        device_count: usize,
    ) -> Self {
        log.record(
            "app_start",
            &[
                ("version", env!("CARGO_PKG_VERSION")),
                ("os", std::env::consts::OS),
            ],
        );
        let mut diagnostics = Self { log, last: None };
        diagnostics.observe(connection, stream, fault, device_count);
        diagnostics
    }

    pub fn record_command(&self, name: &str) {
        self.log.record("command", &[("name", name)]);
    }

    pub fn record_stream_intent(&self, pending: Option<StreamAction>) {
        let pending = match pending {
            Some(StreamAction::Start) => "Start",
            Some(StreamAction::Stop) => "Stop",
            None => "None",
        };
        self.log.record("stream_intent", &[("pending", pending)]);
    }

    pub fn record_error(&self, scope: &str, message: &str) {
        self.log
            .record("error", &[("scope", scope), ("message", message)]);
    }

    pub fn observe(
        &mut self,
        connection: &ConnectionState,
        stream: &StreamState,
        fault: &FaultLogState,
        device_count: usize,
    ) {
        let snapshot = RuntimeSnapshot {
            connection: connection.clone(),
            stream: stream.clone(),
            fault: fault.clone(),
            device_count,
        };
        if self.last.as_ref() == Some(&snapshot) {
            return;
        }
        let connection = format!("{:?}", snapshot.connection);
        let stream = format!("{:?}", snapshot.stream);
        let fault = format!("{:?}", snapshot.fault);
        let device_count = snapshot.device_count.to_string();
        self.log.record(
            "runtime_state",
            &[
                ("connection", &connection),
                ("stream", &stream),
                ("fault", &fault),
                ("device_count", &device_count),
            ],
        );
        self.last = Some(snapshot);
    }

    pub fn record_app_stop(&self) {
        self.log.record("app_stop", &[]);
    }
}

fn write_events(mut file: File, receiver: mpsc::Receiver<String>, max_bytes: usize) {
    let mut used = file
        .metadata()
        .map_or(0, |metadata| metadata.len() as usize);
    if used > max_bytes {
        if reset_file(&mut file).is_err() {
            return;
        }
        used = 0;
    }
    while let Ok(mut line) = receiver.recv() {
        fit_line(&mut line, max_bytes);
        if used.saturating_add(line.len()) > max_bytes {
            if reset_file(&mut file).is_err() {
                return;
            }
            used = 0;
        }
        if file.write_all(line.as_bytes()).is_err() || file.flush().is_err() {
            return;
        }
        used = used.saturating_add(line.len());
    }
}

fn reset_file(file: &mut File) -> io::Result<()> {
    file.flush()?;
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

fn fit_line(line: &mut String, max_bytes: usize) {
    if line.len() <= max_bytes {
        return;
    }
    while line.len() >= max_bytes {
        line.pop();
    }
    line.push('\n');
}

fn unix_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn quoted(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\r' | '\n' | '\t' => vec![' '],
            other => vec![other],
        })
        .collect()
}
