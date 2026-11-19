use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use arklog_core::{DeviceLogOutputBatch, DeviceLogRuntime, LogBatchSink};

// The same executable acts as fake HDC on Windows and Unix. This exercises the
// public runtime without requiring a shell, a device, or an installed HDC.
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "--hold-pipe") {
        std::thread::sleep(Duration::from_secs(8));
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "-t") {
        Command::new(std::env::current_exe().unwrap())
            .arg("--hold-pipe")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        println!("descendant-ready");
        if args[2] == "PARENT-EXIT" {
            std::process::exit(7);
        }
        std::thread::sleep(Duration::from_secs(30));
        return;
    }

    for device in ["PARENT-ALIVE", "PARENT-EXIT"] {
        let runtime = DeviceLogRuntime::new(std::env::current_exe().unwrap().to_string_lossy());
        let sink = Arc::new(ReadySink::default());
        let stream = runtime.start_stream(device, sink.clone()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let (ready, condition) = &sink.0;
        let (ready, _) = condition
            .wait_timeout_while(ready.lock().unwrap(), Duration::from_secs(5), |ready| {
                !*ready
            })
            .unwrap();
        assert!(*ready, "fake HDC did not start its descendant");
        drop(ready);
        if device == "PARENT-EXIT" {
            while runtime
                .finished_stream_status(&stream.stream_id)
                .unwrap()
                .is_none()
            {
                assert!(Instant::now() < deadline, "fake HDC did not exit");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let started = Instant::now();
        if device == "PARENT-EXIT" {
            let exit = runtime
                .reap_finished_stream(&stream.stream_id)
                .unwrap()
                .unwrap();
            assert_eq!(exit.code, Some(7));
        } else {
            runtime.stop_stream(&stream.stream_id).unwrap();
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{device}: inherited pipe blocked cleanup"
        );
        assert!(!runtime.has_stream(&stream.stream_id));
        println!("{device}: process tree cleanup passed");
    }
}

#[derive(Default)]
struct ReadySink((Mutex<bool>, Condvar));

impl LogBatchSink for ReadySink {
    fn deliver(&self, batch: DeviceLogOutputBatch) {
        if batch.lines.iter().any(|line| line == "descendant-ready") {
            *self.0 .0.lock().unwrap() = true;
            self.0 .1.notify_all();
        }
    }
}
