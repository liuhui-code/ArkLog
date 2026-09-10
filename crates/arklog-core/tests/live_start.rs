use std::sync::Arc;

use arklog_core::{
    DeviceLogOutputBatch, DeviceLogRuntime, DeviceLogStartError, DeviceLogStartMode, LogBatchSink,
};

#[test]
fn live_only_start_fails_explicitly_when_no_reliable_boundary_is_available() {
    let runtime = DeviceLogRuntime::new("hdc-must-not-be-launched");

    let error = runtime
        .start_stream_with_mode("USB-01", DeviceLogStartMode::LiveOnly, Arc::new(NullSink))
        .expect_err("strict live-only capture is not supported");

    assert!(matches!(
        error,
        DeviceLogStartError::UnsupportedLiveStart { .. }
    ));
}

struct NullSink;

impl LogBatchSink for NullSink {
    fn deliver(&self, _batch: DeviceLogOutputBatch) {}
}
