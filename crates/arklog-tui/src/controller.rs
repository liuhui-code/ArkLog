use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arklog_core::{
    DeviceFaultLogFetchResult, DeviceFaultLogRawEntry, DeviceLogDevice, DeviceLogOutputBatch,
    DeviceLogRuntime, HdcClient, LogBatchSink,
};

use crate::ArkLogState;

const LOG_CHANNEL_BATCHES: usize = 8;

pub struct ArkLogController {
    state: ArkLogState,
    hdc: HdcClient,
    runtime: Arc<DeviceLogRuntime>,
    devices: Vec<DeviceLogDevice>,
    selected_device: usize,
    batch_sender: SyncSender<DeviceLogOutputBatch>,
    batch_receiver: Receiver<DeviceLogOutputBatch>,
    active_stream: Option<String>,
    runtime_status: String,
    fault_result: Option<DeviceFaultLogFetchResult>,
    selected_fault: usize,
}

impl ArkLogController {
    pub fn discover(executable: impl Into<String>) -> Result<Self, String> {
        let executable = executable.into();
        let hdc = HdcClient::new(executable.clone());
        let devices = hdc.list_devices().unwrap_or_default();
        Self::new(executable, devices)
    }

    pub fn new(
        executable: impl Into<String>,
        devices: Vec<DeviceLogDevice>,
    ) -> Result<Self, String> {
        let executable = executable.into();
        let (batch_sender, batch_receiver) = mpsc::sync_channel(LOG_CHANNEL_BATCHES);
        let runtime_status = if devices.is_empty() {
            "No devices".to_string()
        } else {
            "Ready".to_string()
        };
        Ok(Self {
            state: ArkLogState::new().map_err(|error| error.to_string())?,
            hdc: HdcClient::new(executable.clone()),
            runtime: Arc::new(DeviceLogRuntime::new(executable)),
            devices,
            selected_device: 0,
            batch_sender,
            batch_receiver,
            active_stream: None,
            runtime_status,
            fault_result: None,
            selected_fault: 0,
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

    pub fn runtime_status(&self) -> &str {
        &self.runtime_status
    }

    pub fn is_streaming(&self) -> bool {
        self.active_stream.is_some()
    }

    pub fn start_stream(&mut self) -> Result<(), String> {
        if self.is_streaming() {
            return Ok(());
        }
        let device = self
            .selected_device()
            .ok_or_else(|| "No devices".to_string())?;
        if device.status != "online" {
            return Err(format!("Device {} is {}", device.id, device.status));
        }
        let device_id = device.id.clone();
        self.state.clear().map_err(|error| error.to_string())?;
        let sink = Arc::new(ChannelLogSink {
            sender: self.batch_sender.clone(),
        });
        let stream = self.runtime.start_stream(&device_id, sink)?;
        self.active_stream = Some(stream.stream_id);
        self.runtime_status = "Streaming".to_string();
        Ok(())
    }

    pub fn stop_stream(&mut self) -> Result<(), String> {
        let Some(stream_id) = self.active_stream.clone() else {
            return Ok(());
        };
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
        self.active_stream = None;
        self.runtime_status = "Stopped".to_string();
        stop_result
    }

    pub fn pump_log_batches(&mut self) -> Result<(), String> {
        loop {
            match self.batch_receiver.try_recv() {
                Ok(batch) if self.active_stream.as_deref() == Some(&batch.stream_id) => {
                    self.state
                        .append_lines(batch.lines)
                        .map_err(|error| error.to_string())?;
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err("Log channel disconnected".to_string());
                }
            }
        }
    }

    pub fn refresh_devices(&mut self) -> Result<(), String> {
        if self.is_streaming() {
            return Err("Stop HiLog before refreshing devices".to_string());
        }
        self.devices = self.hdc.list_devices()?;
        self.selected_device = self
            .selected_device
            .min(self.devices.len().saturating_sub(1));
        self.fault_result = None;
        self.selected_fault = 0;
        self.runtime_status = if self.devices.is_empty() {
            "No devices".to_string()
        } else {
            "Ready".to_string()
        };
        Ok(())
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
        let result = self.hdc.list_fault_logs(&device_id)?;
        self.selected_fault = 0;
        self.runtime_status = result.message.clone();
        self.fault_result = Some(result);
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
        self.state.clear().map_err(|error| error.to_string())?;
        self.fault_result = None;
        self.selected_fault = 0;
        self.runtime_status = "Ready".to_string();
        Ok(())
    }
}

impl Drop for ArkLogController {
    fn drop(&mut self) {
        let _ = self.stop_stream();
    }
}

struct ChannelLogSink {
    sender: SyncSender<DeviceLogOutputBatch>,
}

impl LogBatchSink for ChannelLogSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        let _ = self.sender.send(batch);
    }
}
