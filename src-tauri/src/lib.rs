use std::sync::Arc;

use arklog_core::{
    DeviceFaultLogExportResult, DeviceFaultLogExporter, DeviceFaultLogFetchResult,
    DeviceFaultLogRawEntry, DeviceLogDevice, DeviceLogOutputBatch, DeviceLogRuntime,
    DeviceLogStreamSummary, HdcClient, LogBatchSink,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

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
fn list_device_fault_logs(device_id: String) -> Result<DeviceFaultLogFetchResult, String> {
    HdcClient::new(hdc_executable()).list_fault_logs(&device_id)
}

#[tauri::command]
fn export_device_fault_logs(
    device_id: String,
    entries: Vec<DeviceFaultLogRawEntry>,
    app: AppHandle,
) -> Result<Option<DeviceFaultLogExportResult>, String> {
    let default_name = format!("arklog-{}-faults.txt", safe_filename_part(&device_id));
    let Some(file_path) = app
        .dialog()
        .file()
        .set_file_name(default_name)
        .add_filter("Text", &["txt"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|error| format!("Invalid export path: {error}"))?;
    DeviceFaultLogExporter
        .export(&path, &device_id, &entries)
        .map(Some)
}

#[tauri::command]
fn start_device_log_stream(
    device_id: String,
    app: AppHandle,
    runtime: State<'_, DeviceLogRuntime>,
) -> Result<DeviceLogStreamSummary, String> {
    let sink: Arc<dyn LogBatchSink> = Arc::new(TauriEventSink { app });
    runtime.start_stream(&device_id, sink)
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

fn safe_filename_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "device".to_string()
    } else {
        sanitized
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DeviceLogRuntime::new(hdc_executable()))
        .invoke_handler(tauri::generate_handler![
            list_device_log_devices,
            list_device_fault_logs,
            export_device_fault_logs,
            start_device_log_stream,
            stop_device_log_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running ArkLog");
}
