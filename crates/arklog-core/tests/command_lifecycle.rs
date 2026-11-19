use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use arklog_core::{CommandPolicy, CommandRunner, SystemCommandRunner};

#[test]
fn deadline_covers_output_pipes_after_the_parent_exits() {
    let started = Instant::now();
    let error = SystemCommandRunner
        .output_with_policy(
            std::env::current_exe().unwrap().to_str().unwrap(),
            &[
                "--ignored".into(),
                "--exact".into(),
                "exited_parent_with_inherited_pipes".into(),
                "--nocapture".into(),
            ],
            CommandPolicy::new(Duration::from_secs(2), 4096),
        )
        .err();

    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the command deadline must include waiting for inherited output pipes"
    );
    assert!(error.is_some_and(|error| error.contains("timed out")));
}

#[test]
#[ignore = "subprocess fixture"]
fn exited_parent_with_inherited_pipes() {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "holds_inherited_pipes",
            "--nocapture",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    std::process::exit(0);
}

#[test]
#[ignore = "subprocess fixture"]
fn holds_inherited_pipes() {
    std::thread::sleep(Duration::from_secs(8));
}
