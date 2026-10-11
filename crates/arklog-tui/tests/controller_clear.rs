#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use arklog::{ArkLogController, StreamState};

#[test]
fn clear_keeps_the_same_live_stream_and_displays_logs_sent_after_clear() {
    let fixture = ClearHdc::new();
    let mut controller =
        ArkLogController::discover(fixture.executable.to_string_lossy()).expect("controller");
    controller.start_stream().expect("start stream");
    let stream_id = controller.active_stream_id().expect("stream").to_string();
    let deadline = Instant::now() + Duration::from_secs(10);
    while controller.state().raw_count() == 0 && Instant::now() < deadline {
        controller
            .pump_log_batches_limited(8)
            .expect("initial logs");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(controller.state().raw_count(), 1);
    controller.state_mut().scroll_up(1, 1).expect("scroll");
    controller.state_mut().set_find("before").expect("find");

    controller.state_mut().clear().expect("clear live session");
    assert_eq!(controller.state().raw_count(), 0);
    assert!(controller.state().is_following_latest());
    fs::write(fixture.directory.join("hdc.after-clear"), b"").expect("release new logs");
    let deadline = Instant::now() + Duration::from_secs(3);
    while controller.state().raw_count() == 0 && Instant::now() < deadline {
        controller.pump_log_batches_limited(8).expect("new logs");
        controller
            .pump_background_tasks()
            .expect("health after clear");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        controller
            .state_mut()
            .visible_window(10)
            .expect("clear window"),
        ["after-clear"]
    );
    assert_eq!(controller.active_stream_id(), Some(stream_id.as_str()));
    assert_eq!(controller.stream_state(), &StreamState::Streaming);
    assert_eq!(
        fs::read_to_string(fixture.directory.join("hdc.starts"))
            .expect("start count")
            .lines()
            .count(),
        1
    );
    controller.stop_stream().expect("cleanup stream");
}

struct ClearHdc {
    directory: PathBuf,
    executable: PathBuf,
}

impl ClearHdc {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("arklog-clear-hdc-{}", std::process::id()));
        fs::create_dir(&directory).expect("fixture directory");
        let executable = directory.join("hdc");
        fs::write(
            &executable,
            r#"#!/bin/sh
if [ "$1 $2 $3" = "list targets -v" ]; then
  printf 'USB-01\tConnected\n'
  exit 0
fi
if [ "$1 $2 $3" = "-t USB-01 hilog" ]; then
  printf 'start\n' >> "$0.starts"
  printf 'before-clear\n'
  while [ ! -f "$0.after-clear" ]; do sleep 0.01; done
  printf 'after-clear\n'
  exec sleep 30
fi
exit 1
"#,
        )
        .expect("fake HDC script");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("executable fixture");
        Self {
            directory,
            executable,
        }
    }
}

impl Drop for ClearHdc {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
