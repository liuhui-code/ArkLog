use std::sync::Arc;

use arklog_core::{
    DeviceLogDevice, DeviceLogOutputBatch, DeviceLogRuntime, DeviceLogStreamSummary, HdcClient,
    LogBatchSink,
};
use tauri::{AppHandle, Emitter, State};

const DEVICE_LOG_OUTPUT_EVENT: &str = "device-log-output";

struct TauriEventSink {
    app: AppHandle,
}

impl LogBatchSink for TauriEventSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        let _ = self.app.emit(DEVICE_LOG_OUTPUT_EVENT, batch);
    }
}

#[tauri::command]
fn list_device_log_devices() -> Result<Vec<DeviceLogDevice>, String> {
    HdcClient::new(hdc_executable()).list_devices()
}

#[tauri::command]
fn start_device_log_stream(
    device_id: String,
    app: AppHandle,
    runtime: State<'_, DeviceLogRuntime>,
) -> Result<DeviceLogStreamSummary, String> {
    runtime.start_stream(&device_id, Arc::new(TauriEventSink { app }))
}

#[tauri::command]
fn stop_device_log_stream(
    stream_id: String,
    runtime: State<'_, DeviceLogRuntime>,
) -> Result<(), String> {
    runtime.stop_stream(&stream_id)
}

fn hdc_executable() -> String {
    std::env::var("ARKLOG_HDC_PATH").unwrap_or_else(|_| "hdc".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DeviceLogRuntime::new(hdc_executable()))
        .invoke_handler(tauri::generate_handler![
            list_device_log_devices,
            start_device_log_stream,
            stop_device_log_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running ArkLog");
}
