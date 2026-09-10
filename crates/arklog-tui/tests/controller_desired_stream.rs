use arklog::{ArkLogController, ConnectionState, DesiredStream, StreamState};
use arklog_core::DeviceLogDevice;

#[test]
fn running_desire_survives_temporary_device_unavailability() {
    let mut controller = ArkLogController::new("hdc", Vec::new()).expect("controller");

    controller.request_stream_running();
    assert!(!controller.reconcile_stream().expect("wait for device"));

    assert_eq!(controller.desired_stream(), DesiredStream::Running);
    assert_eq!(controller.stream_state(), &StreamState::Stopped);
}

#[test]
fn running_desire_survives_a_stream_start_failure() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new("arklog-hdc-that-does-not-exist", devices).expect("controller");

    controller.request_stream_running();
    controller
        .reconcile_stream()
        .expect_err("missing executable must fail to start");

    assert_eq!(controller.desired_stream(), DesiredStream::Running);
    assert!(matches!(
        controller.stream_state(),
        StreamState::Error { active: false, .. }
    ));
}

#[test]
fn consecutive_start_failures_are_not_retried_in_a_hot_loop() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new("arklog-hdc-that-does-not-exist", devices).expect("controller");
    controller.request_stream_running();

    controller
        .reconcile_stream()
        .expect_err("first start attempt must fail");

    assert!(!controller
        .reconcile_stream()
        .expect("immediate reconcile must observe retry backoff"));
}

#[test]
fn initial_multiple_online_devices_require_an_explicit_selection() {
    let devices = ["USB-A", "USB-B"]
        .into_iter()
        .map(|id| DeviceLogDevice {
            id: id.to_string(),
            label: id.to_string(),
            status: "online".to_string(),
            detail: format!("{id} Connected"),
        })
        .collect();

    let controller = ArkLogController::new("hdc", devices).expect("controller");

    assert_eq!(
        controller.connection_state(),
        &ConnectionState::SelectionRequired
    );
    assert!(controller.selected_device().is_none());
}
