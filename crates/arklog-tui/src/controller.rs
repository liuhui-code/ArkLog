use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use arklog_core::{
    DeviceFaultLogFetchResult, DeviceFaultLogRawEntry, DeviceFaultLogStatus, DeviceLogDevice,
    DeviceLogOutputBatch, DeviceLogRuntime, DeviceLogStreamExit, HdcClient,
};

use crate::background_job::BackgroundJob;
use crate::{
    ArkLogState, ConnectionState, DesiredStream, FaultLogState, StreamAction, StreamState,
};

mod background;
mod stream_health;
mod stream_sink;
use stream_sink::ChannelLogSink;

const LOG_CHANNEL_BATCHES: usize = 8;
const NO_DEVICE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const STREAM_START_RETRY_DELAYS: [Duration; 5] = [
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(5),
];

pub struct ArkLogController {
    state: ArkLogState,
    executable: String,
    hdc: HdcClient,
    runtime: Arc<DeviceLogRuntime>,
    devices: Vec<DeviceLogDevice>,
    selected_device: usize,
    batch_sender: SyncSender<DeviceLogOutputBatch>,
    batch_receiver: Receiver<DeviceLogOutputBatch>,
    active_stream: Option<String>,
    fault_result: Option<DeviceFaultLogFetchResult>,
    selected_fault: usize,
    connection_state: ConnectionState,
    desired_stream: DesiredStream,
    stream_state: StreamState,
    device_refresh: BackgroundJob<Vec<DeviceLogDevice>>,
    next_device_refresh_at: Option<Instant>,
    start_retry_device: Option<String>,
    consecutive_start_failures: usize,
    start_retry_at: Option<Instant>,
    fault_state: FaultLogState,
    fault_refresh: BackgroundJob<DeviceFaultLogFetchResult>,
    fault_refresh_device_id: Option<String>,
    stream_stop: BackgroundJob<()>,
    stopping_stream: Option<String>,
    stream_reap: BackgroundJob<DeviceLogStreamExit>,
}

impl ArkLogController {
    pub fn discover(executable: impl Into<String>) -> Result<Self, String> {
        let executable = executable.into();
        let hdc = HdcClient::new(executable.clone());
        match hdc.list_devices() {
            Ok(devices) => Self::new(executable, devices),
            Err(error) => {
                let mut controller = Self::new(executable, Vec::new())?;
                controller.connection_state = ConnectionState::Error(error.clone());
                Ok(controller)
            }
        }
    }

    pub fn new(
        executable: impl Into<String>,
        devices: Vec<DeviceLogDevice>,
    ) -> Result<Self, String> {
        let executable = executable.into();
        let runtime = Arc::new(DeviceLogRuntime::new(executable.clone()));
        Self::with_runtime(executable, devices, runtime)
    }

    pub fn with_runtime(
        executable: impl Into<String>,
        devices: Vec<DeviceLogDevice>,
        runtime: Arc<DeviceLogRuntime>,
    ) -> Result<Self, String> {
        let executable = executable.into();
        let (batch_sender, batch_receiver) = mpsc::sync_channel(LOG_CHANNEL_BATCHES);
        let online_devices = devices
            .iter()
            .enumerate()
            .filter(|(_, device)| device.status == "online")
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let (connection_state, selected_device) = match online_devices.as_slice() {
            [] => (ConnectionState::NoDevices, 0),
            [index] => (ConnectionState::Ready, *index),
            _ => (ConnectionState::SelectionRequired, devices.len()),
        };
        Ok(Self {
            state: ArkLogState::new().map_err(|error| error.to_string())?,
            executable: executable.clone(),
            hdc: HdcClient::new(executable.clone()),
            runtime,
            devices,
            selected_device,
            batch_sender,
            batch_receiver,
            active_stream: None,
            fault_result: None,
            selected_fault: 0,
            connection_state,
            desired_stream: DesiredStream::Stopped,
            stream_state: StreamState::Stopped,
            device_refresh: BackgroundJob::new(),
            next_device_refresh_at: None,
            start_retry_device: None,
            consecutive_start_failures: 0,
            start_retry_at: None,
            fault_state: FaultLogState::Idle,
            fault_refresh: BackgroundJob::new(),
            fault_refresh_device_id: None,
            stream_stop: BackgroundJob::new(),
            stopping_stream: None,
            stream_reap: BackgroundJob::new(),
        })
    }

    pub fn state(&self) -> &ArkLogState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ArkLogState {
        &mut self.state
    }

    pub fn devices(&self) -> &[DeviceLogDevice] {
        &self.devices
    }

    pub fn selected_device(&self) -> Option<&DeviceLogDevice> {
        self.devices.get(self.selected_device)
    }

    pub fn selected_device_index(&self) -> usize {
        self.selected_device
    }

    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    pub fn stream_state(&self) -> &StreamState {
        &self.stream_state
    }

    pub fn desired_stream(&self) -> DesiredStream {
        self.desired_stream
    }

    pub fn request_stream_running(&mut self) {
        self.desired_stream = DesiredStream::Running;
        if !self.devices.iter().any(|device| device.status == "online")
            && !self.device_refresh.is_running()
            && self.next_device_refresh_at.is_none()
        {
            self.next_device_refresh_at = Some(Instant::now() + NO_DEVICE_REFRESH_INTERVAL);
        }
    }

    pub fn request_stream_stopped(&mut self) {
        self.desired_stream = DesiredStream::Stopped;
        self.next_device_refresh_at = None;
        self.clear_start_retry();
    }

    pub fn toggle_stream(&mut self) -> Result<bool, String> {
        match self.desired_stream {
            DesiredStream::Running => self.request_stream_stopped(),
            DesiredStream::Stopped => self.request_stream_running(),
        }
        self.reconcile_stream()
    }

    pub fn pending_stream_action(&self) -> Option<StreamAction> {
        pending_stream_action(self.desired_stream, &self.stream_state)
    }

    pub fn reconcile_stream(&mut self) -> Result<bool, String> {
        if self.desired_stream == DesiredStream::Stopped {
            if self.active_stream.is_some()
                && !self.stream_stop.is_running()
                && !self.stream_reap.is_running()
            {
                self.request_stream_cleanup(true)?;
                return Ok(true);
            }
            return Ok(false);
        }
        if matches!(self.stream_state, StreamState::Error { active: true, .. })
            && !self.stream_stop.is_running()
            && !self.stream_reap.is_running()
        {
            self.request_stream_cleanup(false)?;
            return Ok(true);
        }
        let selected_device_is_online = self
            .selected_device()
            .is_some_and(|device| device.status == "online");
        if !selected_device_is_online
            || self.active_stream.is_some()
            || self.device_refresh.is_running()
            || self.stream_stop.is_running()
            || self.stream_reap.is_running()
            || self
                .start_retry_at
                .is_some_and(|retry_at| Instant::now() < retry_at)
        {
            return Ok(false);
        }
        self.start_stream_now()?;
        Ok(true)
    }

    pub fn fault_state(&self) -> &FaultLogState {
        &self.fault_state
    }

    pub fn active_stream_id(&self) -> Option<&str> {
        self.active_stream.as_deref()
    }

    pub fn is_streaming(&self) -> bool {
        self.active_stream.is_some()
    }

    pub fn stream_is_live(&self) -> bool {
        self.stream_state == StreamState::Streaming
    }

    pub fn start_stream(&mut self) -> Result<(), String> {
        self.desired_stream = DesiredStream::Running;
        self.start_stream_now()
    }

    fn start_stream_now(&mut self) -> Result<(), String> {
        if self.is_streaming() {
            return Ok(());
        }
        self.stream_state = StreamState::Starting;
        let Some(device) = self.selected_device() else {
            let error = self.connection_state.message().to_string();
            self.stream_state = StreamState::Error {
                message: error.clone(),
                active: false,
            };
            return Err(error);
        };
        if device.status != "online" {
            let error = format!("Device {} is {}", device.id, device.status);
            self.stream_state = StreamState::Error {
                message: error.clone(),
                active: false,
            };
            return Err(error);
        }
        let device_id = device.id.clone();
        self.state.clear().map_err(|error| error.to_string())?;
        let sink = Arc::new(ChannelLogSink {
            sender: self.batch_sender.clone(),
        });
        let stream = match self.runtime.start_stream(&device_id, sink) {
            Ok(stream) => stream,
            Err(error) => {
                self.record_start_failure(&device_id);
                self.stream_state = StreamState::Error {
                    message: error.clone(),
                    active: false,
                };
                return Err(error);
            }
        };
        self.clear_start_retry();
        self.next_device_refresh_at = None;
        self.connection_state = ConnectionState::Ready;
        self.active_stream = Some(stream.stream_id);
        self.stream_state = StreamState::Streaming;
        Ok(())
    }

    pub fn stop_stream(&mut self) -> Result<(), String> {
        self.desired_stream = DesiredStream::Stopped;
        self.stop_stream_now(true)
    }

    fn stop_stream_now(&mut self, preserve_tail: bool) -> Result<(), String> {
        if self.stream_stop.is_running() {
            return Err("Stream stop is already running".to_string());
        }
        while self.stream_reap.is_running() {
            self.pump_log_batches()?;
            self.poll_stream_health()?;
            thread::sleep(Duration::from_millis(10));
        }
        let Some(stream_id) = self.active_stream.clone() else {
            self.stream_state = StreamState::Stopped;
            return Ok(());
        };
        self.stream_state = StreamState::Stopping;
        if !preserve_tail {
            self.active_stream = None;
        }
        let runtime = Arc::clone(&self.runtime);
        let stopping_id = stream_id.clone();
        let (done_sender, done_receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let _ = done_sender.send(runtime.stop_stream(&stopping_id));
        });
        let stop_result = loop {
            self.pump_log_batches()?;
            match done_receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(result) => break result,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err("Log stop worker disconnected".to_string());
                }
            }
        };
        self.pump_log_batches()?;
        self.apply_stop_result(&stream_id, stop_result)
    }

    pub fn request_stop_stream(&mut self) -> Result<(), String> {
        self.desired_stream = DesiredStream::Stopped;
        self.request_stream_cleanup(true)
    }

    fn request_stream_cleanup(&mut self, preserve_tail: bool) -> Result<(), String> {
        if self.stream_stop.is_running() || self.stream_reap.is_running() {
            if !preserve_tail {
                self.active_stream = None;
            }
            return Ok(());
        }
        let Some(stream_id) = self.active_stream.clone() else {
            self.stream_state = StreamState::Stopped;
            return Ok(());
        };
        let runtime = Arc::clone(&self.runtime);
        self.stopping_stream = Some(stream_id.clone());
        self.stream_stop
            .start(move || runtime.stop_stream(&stream_id))?;
        if !preserve_tail {
            self.active_stream = None;
        }
        self.stream_state = StreamState::Stopping;
        Ok(())
    }

    pub fn pump_log_batches(&mut self) -> Result<(), String> {
        self.pump_log_batches_limited(usize::MAX).map(|_| ())
    }

    pub fn pump_log_batches_limited(&mut self, max_batches: usize) -> Result<usize, String> {
        let mut processed = 0;
        while processed < max_batches {
            match self.batch_receiver.try_recv() {
                Ok(batch) if self.active_stream.as_deref() == Some(&batch.stream_id) => {
                    self.state
                        .append_lines(batch.lines)
                        .map_err(|error| error.to_string())?;
                    processed += 1;
                }
                Ok(_) => processed += 1,
                Err(TryRecvError::Empty) => return Ok(processed),
                Err(TryRecvError::Disconnected) => {
                    return Err("Log channel disconnected".to_string());
                }
            }
        }
        Ok(processed)
    }

    pub fn refresh_devices(&mut self) -> Result<(), String> {
        let devices = match self.hdc.list_devices() {
            Ok(devices) => devices,
            Err(error) => {
                self.connection_state = ConnectionState::Error(error.clone());
                return Err(error);
            }
        };
        self.apply_discovered_devices(devices, false)
    }

    pub fn request_device_refresh(&mut self) -> Result<(), String> {
        if self.device_refresh.is_running() {
            return Ok(());
        }
        let executable = self.executable.clone();
        self.device_refresh
            .start(move || HdcClient::new(executable).list_devices())?;
        self.next_device_refresh_at = None;
        self.connection_state = ConnectionState::Refreshing;
        Ok(())
    }

    fn schedule_next_device_refresh(&mut self) {
        if self.desired_stream != DesiredStream::Running || self.device_refresh.is_running() {
            return;
        }
        self.next_device_refresh_at =
            device_refresh_interval(&self.stream_state).map(|interval| Instant::now() + interval);
    }

    fn request_scheduled_device_refresh(&mut self) -> Result<bool, String> {
        if self.desired_stream != DesiredStream::Running || self.device_refresh.is_running() {
            return Ok(false);
        }
        let Some(deadline) = self.next_device_refresh_at else {
            self.schedule_next_device_refresh();
            return Ok(false);
        };
        if Instant::now() < deadline {
            return Ok(false);
        }
        self.request_device_refresh()?;
        Ok(true)
    }

    fn apply_discovered_devices(
        &mut self,
        devices: Vec<DeviceLogDevice>,
        stop_in_background: bool,
    ) -> Result<(), String> {
        let selected_id = self.selected_device().map(|device| device.id.clone());
        let selected_is_online = selected_id.as_deref().is_some_and(|id| {
            devices
                .iter()
                .any(|device| device.id == id && device.status == "online")
        });
        if self.is_streaming() && !selected_is_online {
            if stop_in_background {
                self.request_stream_cleanup(false)?;
            } else {
                self.stop_stream_now(false)?;
            }
        }
        let online_devices = devices
            .iter()
            .enumerate()
            .filter(|(_, device)| device.status == "online")
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let next_selected_device = if selected_is_online {
            selected_id
                .as_deref()
                .and_then(|id| devices.iter().position(|device| device.id == id))
        } else if online_devices.len() == 1 {
            online_devices.first().copied()
        } else {
            selected_id
                .as_deref()
                .and_then(|id| devices.iter().position(|device| device.id == id))
        }
        .unwrap_or(devices.len());
        let target_changed = selected_id.as_deref()
            != devices
                .get(next_selected_device)
                .map(|device| device.id.as_str());
        self.selected_device = next_selected_device;
        self.devices = devices;
        if target_changed {
            self.clear_start_retry();
            self.fault_result = None;
            self.selected_fault = 0;
            self.fault_state = FaultLogState::Idle;
        }
        self.connection_state = if online_devices.is_empty() {
            ConnectionState::NoDevices
        } else if self
            .selected_device()
            .is_some_and(|device| device.status == "online")
        {
            ConnectionState::Ready
        } else {
            ConnectionState::SelectionRequired
        };
        Ok(())
    }

    fn record_start_failure(&mut self, device_id: &str) {
        if self.start_retry_device.as_deref() == Some(device_id) {
            self.consecutive_start_failures = self.consecutive_start_failures.saturating_add(1);
        } else {
            self.start_retry_device = Some(device_id.to_string());
            self.consecutive_start_failures = 1;
        }
        self.start_retry_at =
            Some(Instant::now() + stream_start_retry_delay(self.consecutive_start_failures));
    }

    fn clear_start_retry(&mut self) {
        self.start_retry_device = None;
        self.consecutive_start_failures = 0;
        self.start_retry_at = None;
    }

    pub fn next_device(&mut self) -> Result<(), String> {
        if self.devices.is_empty() {
            return Ok(());
        }
        self.select_device((self.selected_device + 1) % self.devices.len())
    }

    pub fn previous_device(&mut self) -> Result<(), String> {
        if self.devices.is_empty() {
            return Ok(());
        }
        let index = if self.selected_device == 0 {
            self.devices.len() - 1
        } else {
            self.selected_device - 1
        };
        self.select_device(index)
    }

    pub fn refresh_fault_logs(&mut self) -> Result<(), String> {
        let device_id = self
            .selected_device()
            .ok_or_else(|| "No devices".to_string())?
            .id
            .clone();
        let result = match self.hdc.list_fault_logs(&device_id) {
            Ok(result) => result,
            Err(error) => {
                self.fault_state = FaultLogState::Error(error.clone());
                return Err(error);
            }
        };
        self.apply_fault_result(result);
        Ok(())
    }

    pub fn request_fault_log_refresh(&mut self) -> Result<(), String> {
        if self.fault_refresh.is_running() {
            return Ok(());
        }
        let device_id = self
            .selected_device()
            .ok_or_else(|| self.connection_state.message().to_string())?
            .id
            .clone();
        let executable = self.executable.clone();
        self.fault_refresh
            .start(move || HdcClient::new(executable).list_fault_logs(&device_id))?;
        self.fault_refresh_device_id = Some(
            self.selected_device()
                .expect("device existed when refresh started")
                .id
                .clone(),
        );
        self.fault_state = FaultLogState::Refreshing;
        Ok(())
    }

    pub fn fault_result(&self) -> Option<&DeviceFaultLogFetchResult> {
        self.fault_result.as_ref()
    }

    pub fn selected_fault(&self) -> usize {
        self.selected_fault
    }

    pub fn selected_fault_entry(&self) -> Option<&DeviceFaultLogRawEntry> {
        self.fault_result.as_ref()?.entries.get(self.selected_fault)
    }

    fn apply_fault_result(&mut self, result: DeviceFaultLogFetchResult) {
        self.selected_fault = 0;
        self.fault_state = match result.status {
            DeviceFaultLogStatus::Ready | DeviceFaultLogStatus::Empty => FaultLogState::Ready,
            DeviceFaultLogStatus::Unavailable
            | DeviceFaultLogStatus::Unauthorized
            | DeviceFaultLogStatus::Error => FaultLogState::Error(result.message.clone()),
        };
        self.fault_result = Some(result);
    }

    pub fn next_fault(&mut self) {
        let count = self
            .fault_result
            .as_ref()
            .map_or(0, |result| result.entries.len());
        if count > 0 {
            self.selected_fault = (self.selected_fault + 1) % count;
        }
    }

    pub fn previous_fault(&mut self) {
        let count = self
            .fault_result
            .as_ref()
            .map_or(0, |result| result.entries.len());
        if count > 0 {
            self.selected_fault = if self.selected_fault == 0 {
                count - 1
            } else {
                self.selected_fault - 1
            };
        }
    }

    fn select_device(&mut self, index: usize) -> Result<(), String> {
        if self.is_streaming() {
            return Err("Stop HiLog before changing devices".to_string());
        }
        self.selected_device = index;
        self.connection_state = if self
            .selected_device()
            .is_some_and(|device| device.status == "online")
        {
            ConnectionState::Ready
        } else if self.devices.iter().any(|device| device.status == "online") {
            ConnectionState::SelectionRequired
        } else {
            ConnectionState::NoDevices
        };
        self.state.clear().map_err(|error| error.to_string())?;
        self.fault_result = None;
        self.selected_fault = 0;
        self.fault_state = FaultLogState::Idle;
        Ok(())
    }

    fn apply_stop_result(
        &mut self,
        stream_id: &str,
        result: Result<(), String>,
    ) -> Result<(), String> {
        let active = !stream_id.is_empty() && self.runtime.has_stream(stream_id);
        if active && self.active_stream.is_none() {
            self.active_stream = Some(stream_id.to_string());
        } else if !active && self.active_stream.as_deref() == Some(stream_id) {
            self.active_stream = None;
        }
        match result {
            Ok(()) => {
                self.stream_state = StreamState::Stopped;
                Ok(())
            }
            Err(error) => {
                self.stream_state = StreamState::Error {
                    message: error.clone(),
                    active,
                };
                Err(error)
            }
        }
    }
}

fn stream_start_retry_delay(consecutive_failures: usize) -> Duration {
    let delay_index = consecutive_failures
        .saturating_sub(1)
        .min(STREAM_START_RETRY_DELAYS.len() - 1);
    STREAM_START_RETRY_DELAYS[delay_index]
}

fn device_refresh_interval(stream_state: &StreamState) -> Option<Duration> {
    if stream_state == &StreamState::Streaming {
        None
    } else {
        Some(NO_DEVICE_REFRESH_INTERVAL)
    }
}

fn pending_stream_action(
    desired_stream: DesiredStream,
    stream_state: &StreamState,
) -> Option<StreamAction> {
    match (desired_stream, stream_state) {
        (
            DesiredStream::Running,
            StreamState::Stopped | StreamState::Stopping | StreamState::Error { active: false, .. },
        ) => Some(StreamAction::Start),
        (
            DesiredStream::Stopped,
            StreamState::Starting
            | StreamState::Streaming
            | StreamState::Error { active: true, .. },
        ) => Some(StreamAction::Stop),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{device_refresh_interval, pending_stream_action, stream_start_retry_delay};
    use crate::{DesiredStream, StreamAction, StreamState};
    use std::time::Duration;

    #[test]
    fn stream_start_retry_policy_caps_at_five_seconds() {
        let delays = (1..=7).map(stream_start_retry_delay).collect::<Vec<_>>();

        assert_eq!(
            delays,
            [
                Duration::from_millis(500),
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(5),
                Duration::from_secs(5),
                Duration::from_secs(5),
            ]
        );
    }

    #[test]
    fn discovery_runs_while_waiting_but_not_during_a_healthy_stream() {
        assert_eq!(
            device_refresh_interval(&StreamState::Stopped),
            Some(Duration::from_secs(1))
        );
        assert_eq!(device_refresh_interval(&StreamState::Streaming), None);
    }

    #[test]
    fn stop_requested_during_starting_remains_queued() {
        assert_eq!(
            pending_stream_action(DesiredStream::Stopped, &StreamState::Starting),
            Some(StreamAction::Stop)
        );
    }
}
