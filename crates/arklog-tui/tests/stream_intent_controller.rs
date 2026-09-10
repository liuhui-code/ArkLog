#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use arklog::{ArkLogController, ConnectionState, DesiredStream, StreamAction, StreamState};
use arklog_core::DeviceLogDevice;

#[test]
fn cancelled_auto_start_stays_cancelled_when_delayed_discovery_finishes() {
    let mut controller =
        ArkLogController::new(delayed_fixture().to_string_lossy(), Vec::new()).expect("controller");
    controller
        .request_device_refresh()
        .expect("request delayed refresh");
    controller.request_stream_running();

    assert!(!controller.reconcile_stream().expect("wait for device"));
    assert!(!controller.toggle_stream().expect("cancel start"));

    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.connection_state() == &ConnectionState::Refreshing && Instant::now() < deadline
    {
        controller
            .pump_background_tasks()
            .expect("finish discovery");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.connection_state(), &ConnectionState::Ready);
    assert!(!controller.reconcile_stream().expect("remain stopped"));
    assert_eq!(controller.desired_stream(), DesiredStream::Stopped);
    assert_eq!(controller.stream_state(), &StreamState::Stopped);
    assert!(!controller.is_streaming());
}

#[test]
fn controller_starts_when_discovery_satisfies_the_persistent_running_desire() {
    let mut controller =
        ArkLogController::new(tui_hdc_fixture().to_string_lossy(), Vec::new()).expect("controller");
    controller.request_stream_running();
    controller
        .request_device_refresh()
        .expect("request device refresh");

    let deadline = Instant::now() + Duration::from_secs(3);
    while !controller.is_streaming() && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("controller coordinates discovery and start");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.desired_stream(), DesiredStream::Running);
    assert_eq!(controller.stream_state(), &StreamState::Streaming);
    controller.stop_stream().expect("cleanup stream");
}

#[test]
fn running_without_devices_schedules_discovery_without_duplicate_ui_ownership() {
    let mut controller =
        ArkLogController::new(tui_hdc_fixture().to_string_lossy(), Vec::new()).expect("controller");
    controller.request_stream_running();

    let deadline = Instant::now() + Duration::from_secs(3);
    while !controller.is_streaming() && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("controller drives automatic discovery");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.stream_state(), &StreamState::Streaming);
    controller.stop_stream().expect("cleanup stream");
}

#[test]
fn queued_restart_starts_once_after_the_previous_stream_is_fully_reaped() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(fake_stream_fixture().to_string_lossy(), devices)
        .expect("controller");
    controller.start_stream().expect("initial stream");
    let first_stream = controller
        .active_stream_id()
        .expect("stream id")
        .to_string();
    assert!(controller.toggle_stream().expect("request stop"));
    assert_eq!(controller.stream_state(), &StreamState::Stopping);
    assert!(!controller.toggle_stream().expect("queue restart"));
    assert_eq!(controller.desired_stream(), DesiredStream::Running);
    assert_eq!(
        controller.pending_stream_action(),
        Some(StreamAction::Start)
    );

    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.active_stream_id() == Some(first_stream.as_str()) && Instant::now() < deadline
    {
        controller.pump_background_tasks().expect("finish stop");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.stream_state(), &StreamState::Streaming);
    assert_ne!(controller.active_stream_id(), Some(first_stream.as_str()));
    assert!(!controller.reconcile_stream().expect("no duplicate start"));
    controller.stop_stream().expect("cleanup stream");
}

fn delayed_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/delayed-hdc.sh")
}

fn fake_stream_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../arklog-core/tests/fixtures/fake-hdc.sh")
}

fn tui_hdc_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hdc.sh")
}
