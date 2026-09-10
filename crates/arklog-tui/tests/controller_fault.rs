#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use arklog::{ArkLogController, FaultLogState};
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
fn equivalent_device_refresh_preserves_fault_log_and_selection() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.refresh_fault_logs().expect("fault logs");
    controller.next_fault();

    controller
        .refresh_devices()
        .expect("equivalent device refresh");

    assert_eq!(controller.selected_fault(), 1);
    assert!(controller
        .selected_fault_entry()
        .expect("preserved selected fault")
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

#[test]
fn fault_refresh_returns_immediately_and_completes_as_one_background_job() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(delayed_fixture_path().to_string_lossy(), devices)
        .expect("controller");
    let started = Instant::now();

    controller
        .request_fault_log_refresh()
        .expect("request fault refresh");
    controller
        .request_fault_log_refresh()
        .expect("duplicate request is coalesced");

    assert!(started.elapsed() < Duration::from_millis(50));
    assert_eq!(controller.fault_state(), &FaultLogState::Refreshing);
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.fault_state() == &FaultLogState::Refreshing && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("pump background tasks");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.fault_state(), &FaultLogState::Ready);
    assert!(controller
        .selected_fault_entry()
        .expect("fault entry")
        .raw
        .contains("delayed.app"));
}

#[test]
fn fault_result_from_the_previous_device_is_discarded() {
    let devices = ["USB-01", "USB-02"]
        .into_iter()
        .map(|id| DeviceLogDevice {
            id: id.to_string(),
            label: id.to_string(),
            status: "online".to_string(),
            detail: format!("{id} Connected"),
        })
        .collect();
    let mut controller = ArkLogController::new(delayed_fixture_path().to_string_lossy(), devices)
        .expect("controller");

    controller.next_device().expect("explicitly select USB-02");
    controller
        .request_fault_log_refresh()
        .expect("request fault refresh");
    controller.next_device().expect("select USB-01");

    std::thread::sleep(Duration::from_millis(1_100));
    controller
        .pump_background_tasks()
        .expect("pump background tasks");
    // The selected device has no request/result; USB-02 data must not leak into it.
    assert_eq!(controller.selected_device().expect("device").id, "USB-01");
    assert_eq!(controller.fault_state(), &FaultLogState::Idle);
    assert!(controller.fault_result().is_none());
}

#[test]
fn structured_fault_failure_remains_an_error_state() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(failing_fixture_path().to_string_lossy(), devices)
        .expect("controller");

    controller.refresh_fault_logs().expect("structured failure");

    assert_eq!(
        controller.fault_state(),
        &FaultLogState::Error("Connect server failed: daemon unavailable".to_string())
    );
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hdc.sh")
}

fn delayed_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/delayed-hdc.sh")
}

fn failing_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/failing-hdc.sh")
}
