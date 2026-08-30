#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use arklog::ArkLogController;
use arklog_core::DeviceLogDevice;

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

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../arklog-core/tests/fixtures/fake-hdc.sh")
}
