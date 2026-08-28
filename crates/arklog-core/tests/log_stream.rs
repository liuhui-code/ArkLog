use std::io::Cursor;
use std::sync::{Arc, Mutex};

use arklog_core::{spawn_log_reader, DeviceLogOutputBatch, LogBatchSink};

#[test]
fn flushes_the_final_partial_log_batch_without_loss() {
    let sink = Arc::new(RecordingSink::default());

    let worker = spawn_log_reader(
        "stream-1".to_string(),
        "USB-01".to_string(),
        Cursor::new(b"first\nsecond\n".to_vec()),
        sink.clone(),
    );
    worker.join().expect("reader worker");

    let batches = sink.batches.lock().expect("batches");
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].stream_id, "stream-1");
    assert_eq!(batches[0].device_id, "USB-01");
    assert_eq!(batches[0].lines, ["first", "second"]);
}

#[derive(Default)]
struct RecordingSink {
    batches: Mutex<Vec<DeviceLogOutputBatch>>,
}

impl LogBatchSink for RecordingSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        self.batches.lock().expect("batches").push(batch);
    }
}
