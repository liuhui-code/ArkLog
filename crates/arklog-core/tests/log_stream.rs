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
fn flushes_a_continuous_partial_batch_within_the_batching_budget() {
    let sink = Arc::new(WaitingSink::default());
    let worker = spawn_log_reader(
        "stream-paced".to_string(),
        "USB-01".to_string(),
        PacedReader::new(10, Duration::from_millis(30)),
        sink.clone(),
    );

    let first_batch = sink.wait_for_first_batch(Duration::from_secs(1));

    assert!(
        first_batch.lines.len() < 10,
        "the first batch waited for the continuously arriving input to end"
    );
    worker.join().expect("reader worker");
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

struct PacedReader {
    remaining: usize,
    next_index: usize,
    interval: Duration,
}

impl PacedReader {
    fn new(lines: usize, interval: Duration) -> Self {
        Self {
            remaining: lines,
            next_index: 0,
            interval,
        }
    }
}

impl Read for PacedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Ok(0);
        }
        std::thread::sleep(self.interval);
        let line = format!("paced-{}\n", self.next_index);
        let bytes = line.as_bytes();
        assert!(buffer.len() >= bytes.len());
        buffer[..bytes.len()].copy_from_slice(bytes);
        self.remaining -= 1;
        self.next_index += 1;
        Ok(bytes.len())
    }
}

#[derive(Default)]
struct WaitingSink {
    batches: (Mutex<Vec<DeviceLogOutputBatch>>, Condvar),
}

impl WaitingSink {
    fn wait_for_first_batch(&self, timeout: Duration) -> DeviceLogOutputBatch {
        let (batches, ready) = &self.batches;
        let deadline = Instant::now() + timeout;
        let mut guard = batches.lock().expect("batches");
        while guard.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "timed out waiting for first batch");
            let (next, result) = ready.wait_timeout(guard, remaining).expect("wait batch");
            guard = next;
            assert!(!result.timed_out(), "timed out waiting for first batch");
        }
        guard[0].clone()
    }
}

impl LogBatchSink for WaitingSink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        let (batches, ready) = &self.batches;
        batches.lock().expect("batches").push(batch);
        ready.notify_all();
    }
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
