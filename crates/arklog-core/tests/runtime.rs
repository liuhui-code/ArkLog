#![cfg(unix)]

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use arklog_core::{DeviceLogOutputBatch, DeviceLogRuntime, LogBatchSink};

#[test]
fn starts_hilog_for_the_selected_device_and_stops_it() {
    let sink = Arc::new(RecordingSink::default());
    let runtime = DeviceLogRuntime::new(fixture_path().to_string_lossy());

    let stream = runtime
        .start_stream("USB-01", sink.clone())
        .expect("start stream");

    assert_eq!(stream.device_id, "USB-01");
    assert_eq!(stream.status, "running");
    assert_eq!(
        sink.wait_for_lines(3),
        ["-t USB-01 hilog", "first", "second"]
    );
    runtime.stop_stream(&stream.stream_id).expect("stop stream");
}

#[test]
fn stop_waits_for_the_final_partial_batch_to_be_delivered() {
    let sink = Arc::new(RecordingSink::default());
    let runtime = DeviceLogRuntime::new(fixture_path().to_string_lossy());
    let stream = runtime
        .start_stream("TAIL-01", sink.clone())
        .expect("start stream");

    assert_eq!(sink.wait_for_lines(50).len(), 50);
    runtime.stop_stream(&stream.stream_id).expect("stop stream");

    assert!(sink
        .snapshot()
        .iter()
        .any(|line| line == "tail-before-stop"));
}

#[test]
fn reaps_and_reports_a_stream_that_exited_without_a_stop_request() {
    let sink = Arc::new(RecordingSink::default());
    let runtime = DeviceLogRuntime::new(fixture_path().to_string_lossy());
    let stream = runtime.start_stream("EXIT-01", sink).expect("start stream");
    let deadline = Instant::now() + Duration::from_secs(3);

    let observed = loop {
        if let Some(exit) = runtime
            .finished_stream_status(&stream.stream_id)
            .expect("inspect stream")
        {
            break exit;
        }
        assert!(Instant::now() < deadline, "stream exit was not observed");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(observed.code, Some(7));
    assert!(runtime.has_stream(&stream.stream_id));

    let exit = loop {
        if let Some(exit) = runtime
            .reap_finished_stream(&stream.stream_id)
            .expect("poll stream")
        {
            break exit;
        }
        assert!(Instant::now() < deadline, "stream exit was not observed");
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(!exit.success);
    assert_eq!(exit.code, Some(7));
    assert!(!runtime.has_stream(&stream.stream_id));
}

#[test]
fn rejects_an_empty_device_id_before_launching_hdc() {
    let runtime = DeviceLogRuntime::new("hdc");

    let error = runtime
        .start_stream("  ", Arc::new(RecordingSink::default()))
        .expect_err("empty device must fail");

    assert!(error.contains("device"));
}

#[derive(Default)]
struct RecordingSink {
    batches: (Mutex<Vec<DeviceLogOutputBatch>>, Condvar),
}

impl RecordingSink {
    fn wait_for_lines(&self, count: usize) -> Vec<String> {
        let (batches, ready) = &self.batches;
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut guard = batches.lock().expect("batches");
        while guard.iter().map(|batch| batch.lines.len()).sum::<usize>() < count {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "timed out waiting for log output");
            let (next, result) = ready.wait_timeout(guard, remaining).expect("wait output");
            guard = next;
            assert!(!result.timed_out(), "timed out waiting for log output");
        }
        guard.iter().flat_map(|batch| batch.lines.clone()).collect()
    }

    fn snapshot(&self) -> Vec<String> {
        self.batches
            .0
            .lock()
            .expect("batches")
            .iter()
            .flat_map(|batch| batch.lines.clone())
            .collect()
    }
}

impl LogBatchSink for RecordingSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        let (batches, ready) = &self.batches;
        batches.lock().expect("batches").push(batch);
        ready.notify_all();
    }
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-hdc.sh")
}
