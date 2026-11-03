use std::io::{self, BufReader};
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

use crate::mcp::{read_message, tool_result, write_message, MAX_REQUEST_BYTES};
use crate::{ArkLogController, LogExistsQuery, LogQuery, LogSnapshot};

type SnapshotReply = SyncSender<io::Result<(LogSnapshot, Option<String>)>>;

/// Bounded local IPC for the stdio MCP adapter; one query runs at a time.
pub struct McpRuntime {
    address: SocketAddr,
    requests: Receiver<SnapshotReply>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl McpRuntime {
    pub fn start(port: u16) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let (sender, requests) = mpsc::sync_channel::<SnapshotReply>(1);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(_) => break,
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let result = (|| {
                    // Accepted sockets can inherit the listener's nonblocking mode.
                    // Read complete frames with the configured timeout instead.
                    stream.set_nonblocking(false)?;
                    let request =
                        read_message(&mut BufReader::new(&mut stream), MAX_REQUEST_BYTES)?
                            .ok_or_else(|| io::Error::other("empty request"))?;
                    let name = request["name"].as_str().unwrap_or("");
                    let arguments = request.get("arguments").cloned().unwrap_or(json!({}));
                    let exists_query = if name == "arklog_has_logs" {
                        Some(serde_json::from_value::<LogExistsQuery>(arguments.clone())?)
                    } else {
                        None
                    };
                    let query = match name {
                        "arklog_session" if arguments == json!({}) => None,
                        "arklog_has_logs" => None,
                        "arklog_query_logs" => Some(serde_json::from_value::<LogQuery>(arguments)?),
                        _ => return Err(io::Error::other("unknown tool or invalid arguments")),
                    };
                    let (reply, receiver) = mpsc::sync_channel(1);
                    sender.try_send(reply).map_err(io::Error::other)?;
                    let (snapshot, device_id) = receiver
                        .recv_timeout(Duration::from_secs(2))
                        .map_err(io::Error::other)??;
                    if let Some(query) = exists_query {
                        return Ok(json!({"exists":snapshot.has_logs(&query)?}));
                    }
                    let value = match query {
                        Some(query) => {
                            let mut value = serde_json::to_value(snapshot.query(&query)?)?;
                            value["device_id"] = json!(device_id);
                            value
                        }
                        None => json!({"session_id":snapshot.session_id(),
                            "raw_count":snapshot.raw_count(),"device_id":device_id}),
                    };
                    Ok::<Value, io::Error>(value)
                })();
                let response = match result {
                    Ok(value) => tool_result(value, false),
                    Err(error) => tool_result(json!({"error":error.to_string()}), true),
                };
                let _ = write_message(&mut stream, &response);
            }
        });
        Ok(Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Called from the UI loop: only flush/open files here, never scan history.
    pub fn pump(&self, controller: &mut ArkLogController) {
        if let Ok(reply) = self.requests.try_recv() {
            let device = controller.selected_device().map(|device| device.id.clone());
            let snapshot = controller
                .state_mut()
                .query_snapshot()
                .map(|snapshot| (snapshot, device));
            let _ = reply.try_send(snapshot);
        }
    }
}

impl Drop for McpRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
