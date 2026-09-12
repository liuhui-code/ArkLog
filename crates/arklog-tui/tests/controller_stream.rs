#![cfg(unix)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arklog::{ArkLogController, ConnectionState};
use arklog_core::{DeviceLogDevice, DeviceLogRuntime};

#[test]
fn streams_real_hdc_output_through_the_bounded_controller_channel() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");

    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() < 3 && Instant::now() < deadline {
        controller.pump_log_batches().expect("pump batches");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(controller.is_streaming());
    assert_eq!(controller.state().raw_count(), 3);
    assert_eq!(
        controller
            .state_mut()
            .visible_window(10)
            .expect("visible output"),
        ["-t USB-01 hilog", "first", "second"]
    );
    controller.stop_stream().expect("stop stream");
    assert!(!controller.is_streaming());
}

#[test]
fn frame_pump_processes_only_the_requested_number_of_batches() {
    let devices = vec![DeviceLogDevice {
        id: "TAIL-01".to_string(),
        label: "TAIL-01".to_string(),
        status: "online".to_string(),
        detail: "TAIL-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start stream");

    wait_for_one_batch(&mut controller);
    assert_eq!(controller.state().raw_count(), 50);
    wait_for_one_batch(&mut controller);
    assert_eq!(controller.state().raw_count(), 51);
    controller.stop_stream().expect("stop stream");
}

fn wait_for_one_batch(controller: &mut ArkLogController) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let processed = controller
            .pump_log_batches_limited(1)
            .expect("pump one batch");
        if processed == 1 {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for log batch");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn idle_background_pump_reports_no_visible_activity() {
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), Vec::new()).expect("controller");

    assert!(!controller
        .pump_background_tasks_with_activity()
        .expect("idle background pump"));
}

#[test]
fn initial_selection_prefers_the_first_online_device() {
    let devices = vec![
        DeviceLogDevice {
            id: "USB-OFFLINE".to_string(),
            label: "USB-OFFLINE".to_string(),
            status: "offline".to_string(),
            detail: "USB-OFFLINE USB Offline".to_string(),
        },
        DeviceLogDevice {
            id: "USB-01".to_string(),
            label: "USB-01".to_string(),
            status: "online".to_string(),
            detail: "USB-01 USB Ready".to_string(),
        },
    ];

    let controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");

    assert_eq!(
        controller.selected_device().expect("online device").id,
        "USB-01"
    );
}

#[test]
fn offline_only_snapshot_is_not_ready_to_stream() {
    let devices = vec![DeviceLogDevice {
        id: "USB-OFFLINE".to_string(),
        label: "USB-OFFLINE".to_string(),
        status: "offline".to_string(),
        detail: "USB-OFFLINE USB Offline".to_string(),
    }];

    let controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");

    assert_eq!(controller.connection_state(), &ConnectionState::NoDevices);
}

#[test]
fn refreshes_connected_devices_without_interrupting_the_active_stream() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(device_fixture_path().to_string_lossy(), devices)
        .expect("controller");

    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() == 0 && Instant::now() < deadline {
        controller.pump_log_batches().expect("pump batches");
        std::thread::sleep(Duration::from_millis(10));
    }
    controller.refresh_devices().expect("refresh devices");

    assert!(controller.is_streaming());
    assert_eq!(controller.stream_state(), &arklog::StreamState::Streaming);
    assert_eq!(controller.devices().len(), 1);
    assert_eq!(controller.selected_device().expect("device").id, "USB-01");
    controller.stop_stream().expect("stop stream");
}

#[test]
fn refresh_falls_back_to_an_online_device_when_the_previous_device_disappears() {
    let devices = vec![DeviceLogDevice {
        id: "USB-OLD".to_string(),
        label: "USB-OLD".to_string(),
        status: "online".to_string(),
        detail: "USB-OLD Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(replaced_devices_fixture_path().to_string_lossy(), devices)
            .expect("controller");

    controller.refresh_devices().expect("refresh devices");

    assert_eq!(controller.selected_device().expect("device").id, "USB-NEW");
}

#[test]
fn refresh_keeps_the_current_target_when_it_is_still_online() {
    let devices = vec![DeviceLogDevice {
        id: "USB-OLD".to_string(),
        label: "USB-OLD".to_string(),
        status: "online".to_string(),
        detail: "USB-OLD Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(multiple_devices_fixture_path().to_string_lossy(), devices)
            .expect("controller");

    controller.refresh_devices().expect("refresh devices");

    assert_eq!(controller.selected_device().expect("device").id, "USB-OLD");
}

#[test]
fn refresh_does_not_guess_when_multiple_online_devices_replace_the_target() {
    let devices = vec![DeviceLogDevice {
        id: "USB-MISSING".to_string(),
        label: "USB-MISSING".to_string(),
        status: "online".to_string(),
        detail: "USB-MISSING Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(multiple_devices_fixture_path().to_string_lossy(), devices)
            .expect("controller");

    controller.refresh_devices().expect("refresh devices");

    assert!(controller.selected_device().is_none());
    assert_eq!(
        controller.connection_state(),
        &ConnectionState::SelectionRequired
    );
}

#[test]
fn reports_device_discovery_failure_without_hiding_it_as_an_empty_list() {
    let mut controller = ArkLogController::discover(failing_fixture_path().to_string_lossy())
        .expect("controller remains available for refresh");

    assert!(controller.devices().is_empty());
    assert_eq!(
        controller.connection_state(),
        &ConnectionState::Error("Connect server failed: daemon unavailable".to_string())
    );
    assert_eq!(
        controller.start_stream().expect_err("no connected device"),
        "Connect server failed: daemon unavailable"
    );
}

#[test]
fn stops_the_stale_stream_when_refresh_confirms_the_device_disconnected() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(disconnected_fixture_path().to_string_lossy(), devices)
            .expect("controller");
    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() == 0 && Instant::now() < deadline {
        controller.pump_log_batches().expect("pump batches");
        std::thread::sleep(Duration::from_millis(10));
    }

    controller.refresh_devices().expect("refresh devices");

    assert!(!controller.is_streaming());
    assert!(controller.devices().is_empty());
    assert_eq!(controller.connection_state(), &ConnectionState::NoDevices);
}

#[test]
fn background_disconnect_never_blocks_while_reaping_the_stale_stream() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(disconnected_fixture_path().to_string_lossy(), devices)
            .expect("controller");
    controller.start_stream().expect("start stream");
    controller
        .request_device_refresh()
        .expect("request refresh");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.connection_state() == &ConnectionState::Refreshing && Instant::now() < deadline
    {
        let pump_started = Instant::now();
        controller
            .pump_background_tasks()
            .expect("pump background tasks");
        assert!(pump_started.elapsed() < Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.connection_state(), &ConnectionState::NoDevices);
    assert_eq!(controller.stream_state(), &arklog::StreamState::Stopping);
    assert!(
        !controller.is_streaming(),
        "a disconnected device is fenced immediately while its worker is reaped"
    );

    while controller.stream_state() == &arklog::StreamState::Stopping && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("finish background stop");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.stream_state(), &arklog::StreamState::Stopped);
    assert!(!controller.is_streaming());
}

#[test]
fn refresh_request_returns_immediately_and_applies_devices_in_the_background() {
    let mut controller =
        ArkLogController::new(delayed_fixture_path().to_string_lossy(), Vec::new())
            .expect("controller");
    let started = Instant::now();

    controller
        .request_device_refresh()
        .expect("request refresh");

    assert!(started.elapsed() < Duration::from_millis(50));
    assert_eq!(controller.connection_state(), &ConnectionState::Refreshing);
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.connection_state() == &ConnectionState::Refreshing && Instant::now() < deadline
    {
        controller
            .pump_background_tasks()
            .expect("pump background tasks");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.connection_state(), &ConnectionState::Ready);
    assert_eq!(
        controller.selected_device().expect("device").id,
        "USB-DELAYED"
    );
}

#[test]
fn stop_failure_reports_an_error_instead_of_a_false_stopped_state() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let runtime = Arc::new(DeviceLogRuntime::new(fixture_path().to_string_lossy()));
    let mut controller = ArkLogController::with_runtime(
        fixture_path().to_string_lossy(),
        devices,
        Arc::clone(&runtime),
    )
    .expect("controller");
    controller.start_stream().expect("start stream");
    let stream_id = controller
        .active_stream_id()
        .expect("active stream")
        .to_string();
    runtime.stop_stream(&stream_id).expect("external stop");

    let error = controller.stop_stream().expect_err("second stop must fail");

    assert!(error.contains("Unknown device log stream"));
    assert!(!controller.is_streaming());
    assert!(matches!(
        controller.stream_state(),
        arklog::StreamState::Error { active: false, .. }
    ));
}

#[test]
fn stop_request_keeps_the_controller_responsive_until_the_stream_is_reaped() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start stream");
    let started = Instant::now();

    controller.request_stop_stream().expect("request stop");

    assert!(started.elapsed() < Duration::from_millis(50));
    assert_eq!(controller.stream_state(), &arklog::StreamState::Stopping);
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.stream_state() == &arklog::StreamState::Stopping && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("pump background tasks");
        controller.pump_log_batches().expect("pump final logs");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.stream_state(), &arklog::StreamState::Stopped);
    assert!(!controller.is_streaming());
}

#[test]
fn background_stop_commits_the_final_partial_log_batch() {
    let devices = vec![DeviceLogDevice {
        id: "TAIL-01".to_string(),
        label: "TAIL-01".to_string(),
        status: "online".to_string(),
        detail: "TAIL-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() < 50 && Instant::now() < deadline {
        controller.pump_log_batches().expect("pump initial batch");
        std::thread::sleep(Duration::from_millis(10));
    }

    controller.request_stop_stream().expect("request stop");
    while controller.stream_state() == &arklog::StreamState::Stopping && Instant::now() < deadline {
        controller
            .pump_background_tasks()
            .expect("finish background stop");
        std::thread::sleep(Duration::from_millis(10));
    }
    controller.pump_log_batches().expect("pump final batch");

    assert!(controller
        .state_mut()
        .visible_window(100)
        .expect("visible output")
        .iter()
        .any(|line| line == "tail-before-stop"));
}

#[test]
fn background_stop_drains_a_full_bounded_batch_channel_until_reap_completes() {
    let devices = vec![DeviceLogDevice {
        id: "FULL-STOP".to_string(),
        label: "FULL-STOP".to_string(),
        status: "online".to_string(),
        detail: "FULL-STOP Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start burst stream");
    std::thread::sleep(Duration::from_millis(100));

    controller.request_stop_stream().expect("request stop");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.stream_state() == &arklog::StreamState::Stopping && Instant::now() < deadline {
        controller
            .pump_log_batches_limited(8)
            .expect("drain bounded channel");
        controller
            .pump_background_tasks()
            .expect("poll stop completion");
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(controller.stream_state(), &arklog::StreamState::Stopped);
    assert!(!controller.is_streaming());
}

#[test]
fn device_switch_discards_the_old_streams_late_partial_batch() {
    let devices = vec![DeviceLogDevice {
        id: "USB-A".to_string(),
        label: "USB-A".to_string(),
        status: "online".to_string(),
        detail: "USB-A Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(switching_devices_fixture_path().to_string_lossy(), devices)
            .expect("controller");
    controller.start_stream().expect("start A");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() < 50 && Instant::now() < deadline {
        controller.pump_log_batches().expect("pump initial A batch");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(controller.state().raw_count(), 50);

    controller
        .request_device_refresh()
        .expect("discover replacement device");
    while (controller.connection_state() == &ConnectionState::Refreshing
        || controller.stream_state() == &arklog::StreamState::Stopping
        || controller.state().raw_count() == 0)
        && Instant::now() < deadline
    {
        controller
            .pump_log_batches()
            .expect("drain bounded channel");
        controller
            .pump_background_tasks()
            .expect("finish device switch cleanup");
        std::thread::sleep(Duration::from_millis(5));
    }
    controller.pump_log_batches().expect("drain late batches");

    assert_eq!(controller.selected_device().expect("device").id, "USB-B");
    assert_eq!(controller.desired_stream(), arklog::DesiredStream::Running);
    assert_eq!(controller.stream_state(), &arklog::StreamState::Streaming);
    assert!(controller
        .state_mut()
        .visible_window(100)
        .expect("new device output")
        .iter()
        .any(|line| line == "new-device"));
    assert!(!controller
        .state_mut()
        .visible_window(100)
        .expect("visible output")
        .iter()
        .any(|line| line == "old-tail-after-switch"));
}

#[test]
fn unexpected_hdc_exit_replaces_the_false_live_state_and_keeps_final_logs() {
    let devices = vec![DeviceLogDevice {
        id: "EXIT-01".to_string(),
        label: "EXIT-01".to_string(),
        status: "online".to_string(),
        detail: "EXIT-01 Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);

    while controller.is_streaming() && Instant::now() < deadline {
        controller
            .pump_log_batches_limited(8)
            .expect("drain final logs");
        controller
            .pump_background_tasks()
            .expect("poll stream health");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(matches!(
        controller.stream_state(),
        arklog::StreamState::Error { active: false, .. }
    ));
    assert!(!controller.is_streaming());
    assert!(controller
        .state_mut()
        .visible_window(10)
        .expect("visible output")
        .iter()
        .any(|line| line == "last-before-exit"));
}

#[test]
fn cached_online_device_reconnects_when_post_disconnect_discovery_fails() {
    let marker = std::env::temp_dir().join(format!(
        "arklog-reconnect-after-discovery-error-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker);
    let devices = vec![DeviceLogDevice {
        id: "CACHED-ONLINE".to_string(),
        label: "CACHED-ONLINE".to_string(),
        status: "online".to_string(),
        detail: "CACHED-ONLINE Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(
        reconnecting_after_discovery_error_fixture_path().to_string_lossy(),
        devices,
    )
    .expect("controller");
    controller
        .start_stream()
        .expect("start stream that exits once");
    let first_stream = controller
        .active_stream_id()
        .expect("first stream id")
        .to_string();
    let deadline = Instant::now() + Duration::from_secs(3);

    while (controller.active_stream_id().is_none()
        || controller.active_stream_id() == Some(first_stream.as_str()))
        && Instant::now() < deadline
    {
        controller
            .pump_log_batches_limited(8)
            .expect("drain stream output");
        let _ = controller.pump_background_tasks();
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        controller.selected_device().expect("cached device").status,
        "online"
    );
    assert_eq!(controller.connection_state(), &ConnectionState::Ready);
    assert_eq!(controller.stream_state(), &arklog::StreamState::Streaming);
    assert_ne!(controller.active_stream_id(), Some(first_stream.as_str()));
    let replacement_stream = controller
        .active_stream_id()
        .expect("replacement stream id")
        .to_string();
    let stability_deadline = Instant::now() + Duration::from_millis(1_200);
    while Instant::now() < stability_deadline {
        let _ = controller.pump_background_tasks();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.connection_state(), &ConnectionState::Ready);
    assert_eq!(
        controller.active_stream_id(),
        Some(replacement_stream.as_str()),
        "a stale discovery timer must not disturb the replacement stream"
    );
    controller.stop_stream().expect("cleanup replacement");
    let _ = std::fs::remove_file(marker);
}

#[test]
fn quiet_but_healthy_stream_is_not_reconnected() {
    let devices = vec![DeviceLogDevice {
        id: "USB-01".to_string(),
        label: "USB-01".to_string(),
        status: "online".to_string(),
        detail: "USB-01 Connected".to_string(),
    }];
    let mut controller = ArkLogController::new(device_fixture_path().to_string_lossy(), devices)
        .expect("controller");
    controller.start_stream().expect("start quiet stream");
    let stream_id = controller
        .active_stream_id()
        .expect("stream id")
        .to_string();
    let deadline = Instant::now() + Duration::from_millis(1_200);

    while Instant::now() < deadline {
        controller.pump_log_batches().expect("pump initial output");
        controller
            .pump_background_tasks()
            .expect("health check uses process state, not log silence");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(controller.active_stream_id(), Some(stream_id.as_str()));
    assert_eq!(controller.stream_state(), &arklog::StreamState::Streaming);
    controller.stop_stream().expect("cleanup stream");
}

#[test]
fn late_old_reap_result_cannot_clear_the_replacement_stream() {
    let devices = vec![DeviceLogDevice {
        id: "USB-EXIT".to_string(),
        label: "USB-EXIT".to_string(),
        status: "online".to_string(),
        detail: "USB-EXIT Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(switching_devices_fixture_path().to_string_lossy(), devices)
            .expect("controller");
    controller.start_stream().expect("start exiting stream");
    let retiring_stream = controller
        .active_stream_id()
        .expect("retiring stream id")
        .to_string();
    let deadline = Instant::now() + Duration::from_secs(3);

    while controller.stream_state() == &arklog::StreamState::Streaming && Instant::now() < deadline
    {
        controller
            .pump_background_tasks()
            .expect("observe unexpected exit");
        std::thread::sleep(Duration::from_millis(5));
    }
    controller
        .request_device_refresh()
        .expect("discover replacement while old stream reaps");
    while (controller.active_stream_id().is_none()
        || controller.active_stream_id() == Some(retiring_stream.as_str())
        || controller.state().raw_count() == 0)
        && Instant::now() < deadline
    {
        controller
            .pump_log_batches_limited(8)
            .expect("drain retiring stream");
        controller
            .pump_background_tasks()
            .expect("finish reap and start replacement");
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(controller.selected_device().expect("device").id, "USB-B");
    assert_ne!(
        controller.active_stream_id(),
        Some(retiring_stream.as_str())
    );
    assert_eq!(controller.stream_state(), &arklog::StreamState::Streaming);
    assert!(controller
        .state_mut()
        .visible_window(10)
        .expect("replacement output")
        .iter()
        .any(|line| line == "new-device"));
    controller.stop_stream().expect("cleanup replacement");
}

#[test]
fn exited_stream_reaps_in_the_background_while_the_ui_drains_a_large_tail() {
    let devices = vec![DeviceLogDevice {
        id: "EXIT-BURST".to_string(),
        label: "EXIT-BURST".to_string(),
        status: "online".to_string(),
        detail: "EXIT-BURST Connected".to_string(),
    }];
    let mut controller =
        ArkLogController::new(fixture_path().to_string_lossy(), devices).expect("controller");
    controller.start_stream().expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);

    while controller.is_streaming() && Instant::now() < deadline {
        controller
            .pump_log_batches_limited(8)
            .expect("drain bounded tail");
        controller
            .pump_background_tasks()
            .expect("poll background reap");
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(!controller.is_streaming(), "background reap timed out");
    assert_eq!(controller.state().raw_count(), 1_001);
    assert!(matches!(
        controller.stream_state(),
        arklog::StreamState::Error { active: false, .. }
    ));
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../arklog-core/tests/fixtures/fake-hdc.sh")
}

fn device_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hdc.sh")
}

fn failing_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/failing-hdc.sh")
}

fn disconnected_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/disconnected-hdc.sh")
}

fn delayed_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/delayed-hdc.sh")
}

fn replaced_devices_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replaced-devices-hdc.sh")
}

fn multiple_devices_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multiple-devices-hdc.sh")
}

fn switching_devices_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/switching-devices-hdc.sh")
}

fn reconnecting_after_discovery_error_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/reconnecting-after-discovery-error-hdc.sh")
}
