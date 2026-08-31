use std::sync::mpsc::SyncSender;

use arklog_core::{DeviceLogOutputBatch, LogBatchSink};

use super::ArkLogController;

pub(super) struct ChannelLogSink {
    pub(super) sender: SyncSender<DeviceLogOutputBatch>,
}

impl LogBatchSink for ChannelLogSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        let _ = self.sender.send(batch);
    }
}

impl Drop for ArkLogController {
    fn drop(&mut self) {
        let _ = self.stop_stream();
    }
}
