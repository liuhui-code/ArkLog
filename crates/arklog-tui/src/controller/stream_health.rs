use crate::StreamState;

use super::ArkLogController;

impl ArkLogController {
    pub(super) fn poll_stream_health(&mut self) -> Result<bool, String> {
        if let Some(result) = self.stream_reap.poll() {
            return match result {
                Ok(exit) => {
                    self.pump_log_batches()?;
                    self.active_stream = None;
                    self.stream_state = StreamState::Error {
                        message: exit_message(exit.code),
                        active: false,
                    };
                    Ok(true)
                }
                Err(error) => {
                    let active = self
                        .active_stream
                        .as_deref()
                        .is_some_and(|id| self.runtime.has_stream(id));
                    if !active {
                        self.active_stream = None;
                    }
                    self.stream_state = StreamState::Error {
                        message: error.clone(),
                        active,
                    };
                    Err(error)
                }
            };
        }
        if self.stream_stop.is_running() || self.stream_state != StreamState::Streaming {
            return Ok(false);
        }
        let Some(stream_id) = self.active_stream.clone() else {
            return Ok(false);
        };
        match self.runtime.finished_stream_status(&stream_id) {
            Ok(None) => Ok(false),
            Ok(Some(exit)) => {
                let runtime = self.runtime.clone();
                let reaping_id = stream_id.clone();
                self.stream_reap.start(move || {
                    runtime
                        .reap_finished_stream(&reaping_id)?
                        .ok_or_else(|| format!("HDC HiLog did not exit: {reaping_id}"))
                })?;
                self.stream_state = StreamState::Error {
                    message: exit_message(exit.code),
                    active: true,
                };
                Ok(true)
            }
            Err(error) if !self.runtime.has_stream(&stream_id) => {
                self.active_stream = None;
                self.stream_state = StreamState::Error {
                    message: error,
                    active: false,
                };
                Ok(true)
            }
            Err(error) => Err(error),
        }
    }
}

fn exit_message(code: Option<i32>) -> String {
    let detail = code.map_or_else(
        || "without an exit code".to_string(),
        |code| format!("with code {code}"),
    );
    format!("HDC HiLog exited unexpectedly {detail}")
}
