#![cfg(unix)]

use std::path::PathBuf;

use arklog::ArkLogController;
use arklog_core::DeviceLogDevice;

#[test]
fn refreshes_fault_logs_and_moves_the_raw_inspector_selection() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");

    controller.refresh_fault_logs().expect("fault logs");
    let result = controller.fault_result().expect("fault result");
    assert_eq!(result.entries.len(), 2);
    assert!(result.entries[0].raw.contains("com.example.first"));
    assert_eq!(controller.selected_fault(), 0);

    controller.next_fault();
    assert_eq!(controller.selected_fault(), 1);
    assert!(controller
        .selected_fault_entry()
        .expect("selected entry")
        .raw
        .contains("com.example.second"));
}

#[test]
fn changing_device_clears_session_data_before_the_next_stream() {
    let devices = ["USB-01", "USB-02"]
        .into_iter()
        .map(|id| DeviceLogDevice {
            id: id.to_string(),
            label: id.to_string(),
            status: "online".to_string(),
            detail: format!("{id} Connected"),
        })
        .collect();
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller
        .state_mut()
        .append_lines(["old device log"])
        .expect("old log");

    controller.next_device().expect("next device");

    assert_eq!(controller.selected_device().expect("device").id, "USB-02");
    assert_eq!(controller.state().raw_count(), 0);
    assert!(controller.fault_result().is_none());
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hdc.sh")
}
