#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use arklog::{ArkLogController, ConnectionState, StreamIntent, StreamState};
use arklog_core::DeviceLogDevice;

#[test]
fn cancelled_auto_start_stays_cancelled_when_delayed_discovery_finishes() {
    let mut controller =
        ArkLogController::new(delayed_fixture().to_string_lossy(), Vec::new()).expect("controller");
    controller
        .request_device_refresh()
        .expect("request delayed refresh");
    let mut intent = StreamIntent::auto_start();

    assert!(!intent.reconcile(&mut controller).expect("wait for device"));
    intent.toggle(controller.stream_state());
    assert!(!intent.reconcile(&mut controller).expect("cancel start"));

    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.connection_state() == &ConnectionState::Refreshing && Instant::now() < deadline
    {
        controller
            .pump_background_tasks()
            .expect("finish discovery");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.connection_state(), &ConnectionState::Ready);
    assert!(!intent.reconcile(&mut controller).expect("remain stopped"));
    assert_eq!(controller.stream_state(), &StreamState::Stopped);
    assert!(!controller.is_streaming());
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
    let mut intent = StreamIntent::idle();

    intent.toggle(controller.stream_state());
    assert!(intent.reconcile(&mut controller).expect("request stop"));
    assert_eq!(controller.stream_state(), &StreamState::Stopping);
    intent.toggle(controller.stream_state());

    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.stream_state() == &StreamState::Stopping && Instant::now() < deadline {
        controller.pump_background_tasks().expect("finish stop");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.stream_state(), &StreamState::Stopped);
    assert!(intent
        .reconcile(&mut controller)
        .expect("start replacement"));
    assert_eq!(controller.stream_state(), &StreamState::Streaming);
    assert_ne!(controller.active_stream_id(), Some(first_stream.as_str()));
    assert!(!intent
        .reconcile(&mut controller)
        .expect("no duplicate start"));
    controller.stop_stream().expect("cleanup stream");
}

fn delayed_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/delayed-hdc.sh")
}

fn fake_stream_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../arklog-core/tests/fixtures/fake-hdc.sh")
}
