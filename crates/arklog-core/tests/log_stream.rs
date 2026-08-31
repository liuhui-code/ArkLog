use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

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

#[test]
fn preserves_order_across_a_large_burst_and_its_partial_tail() {
    let sink = Arc::new(RecordingSink::default());
    let expected = (0..10_037)
        .map(|index| format!("line-{index}"))
        .collect::<Vec<_>>();
    let input = format!("{}\n", expected.join("\n"));

    let worker = spawn_log_reader(
        "stream-burst".to_string(),
        "USB-01".to_string(),
        Cursor::new(input.into_bytes()),
        sink.clone(),
    );
    worker.join().expect("reader worker");

    let actual = sink
        .batches
        .lock()
        .expect("batches")
        .iter()
        .flat_map(|batch| batch.lines.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn slow_ui_backpressures_the_reader_before_it_can_buffer_megabytes() {
    let input = (0..5_000)
        .map(|_| format!("{}\n", "x".repeat(1_024)))
        .collect::<String>()
        .into_bytes();
    let bytes_read = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(BlockingSink::default());
    let worker = spawn_log_reader(
        "stream-slow-ui".to_string(),
        "USB-01".to_string(),
        CountingReader {
            cursor: Cursor::new(input),
            bytes_read: Arc::clone(&bytes_read),
        },
        sink.clone(),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !sink.entered.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(sink.entered.load(Ordering::Acquire));
    std::thread::sleep(Duration::from_millis(50));

    assert!(bytes_read.load(Ordering::Relaxed) < 2 * 1024 * 1024);
    sink.release();
    worker.join().expect("reader worker");
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

struct CountingReader {
    cursor: Cursor<Vec<u8>>,
    bytes_read: Arc<AtomicUsize>,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.cursor.read(buffer)?;
        self.bytes_read.fetch_add(count, Ordering::Relaxed);
        Ok(count)
    }
}

#[derive(Default)]
struct BlockingSink {
    entered: AtomicBool,
    released: (Mutex<bool>, Condvar),
}

impl BlockingSink {
    fn release(&self) {
        *self.released.0.lock().expect("released") = true;
        self.released.1.notify_all();
    }
}

impl LogBatchSink for BlockingSink {
    fn deliver(&self, _batch: DeviceLogOutputBatch) {
        self.entered.store(true, Ordering::Release);
        let (released, ready) = &self.released;
        let mut guard = released.lock().expect("released");
        while !*guard {
            guard = ready.wait(guard).expect("release wait");
        }
    }
}
