#[cfg(unix)]
use std::path::PathBuf;
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

#[cfg(unix)]
use arklog_core::SystemCommandRunner;
use arklog_core::{CommandOutput, CommandPolicy, CommandRunner, HdcClient};

#[test]
fn lists_structured_devices_from_hdc_targets() {
    let client = HdcClient::with_runner("hdc", FixtureRunner);

    let devices = client.list_devices().expect("devices");

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].id, "USB-01");
    assert_eq!(devices[0].label, "USB-01");
    assert_eq!(devices[0].status, "online");
    assert_eq!(devices[1].id, "USB-02");
    assert_eq!(devices[1].label, "USB-02");
    assert_eq!(devices[1].status, "offline");
}

#[test]
fn prefixes_online_device_ids_with_the_special_cust_phone_code() {
    let client = HdcClient::with_runner("hdc", PhoneCodeRunner);

    let devices = client.list_devices().expect("devices");

    assert_eq!(devices[0].id, "USB-01");
    assert_eq!(devices[0].label, "TAS-AL00 USB-01");
}

#[test]
#[cfg(unix)]
fn device_discovery_kills_a_command_that_exceeds_its_deadline() {
    let policy = CommandPolicy::new(Duration::from_millis(100), 4 * 1024);
    let client = HdcClient::with_policies(
        slow_fixture_path().to_string_lossy(),
        SystemCommandRunner,
        policy,
        policy,
    );
    let started = Instant::now();

    let error = client.list_devices().expect_err("slow discovery must fail");

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.to_ascii_lowercase().contains("timed out"));
}

#[test]
fn device_discovery_rejects_output_above_its_memory_budget() {
    let policy = CommandPolicy::new(Duration::from_secs(1), 16);
    let client = HdcClient::with_policies("hdc", OversizedRunner, policy, policy);

    let error = client
        .list_devices()
        .expect_err("oversized discovery must fail");

    assert!(error.contains("exceeded 16 bytes"));
}

#[test]
#[cfg(unix)]
fn device_discovery_kills_a_still_running_command_as_soon_as_output_overflows() {
    let policy = CommandPolicy::new(Duration::from_secs(3), 64);
    let client = HdcClient::with_policies(
        oversized_fixture_path().to_string_lossy(),
        SystemCommandRunner,
        policy,
        policy,
    );
    let started = Instant::now();

    let error = client
        .list_devices()
        .expect_err("oversized command must fail");

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.contains("exceeded 64 bytes"));
}

#[test]
fn ignores_diagnostics_that_do_not_match_a_device_record() {
    let client = HdcClient::with_runner("hdc", DiagnosticRunner);

    let devices = client.list_devices().expect("devices");

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "USB-01");
}

#[test]
fn rejects_connected_diagnostics_and_does_not_repeat_device_fields_in_detail() {
    let client = HdcClient::with_runner("hdc", ConnectedDiagnosticRunner);

    let devices = client.list_devices().expect("devices");

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "USB-01");
    assert_eq!(devices[0].status, "online");
    assert_eq!(devices[0].detail, "USB Phone localhost hdc");
}

#[test]
fn parses_official_verbose_target_fields_and_connection_states() {
    let client = HdcClient::with_runner("hdc", OfficialVerboseRunner);

    let devices = client.list_devices().expect("verbose devices");

    assert_eq!(devices.len(), 3);
    assert_eq!(
        (devices[0].id.as_str(), devices[0].status.as_str()),
        ("USB-01", "online")
    );
    assert_eq!(
        (devices[1].id.as_str(), devices[1].status.as_str()),
        ("192.0.2.1:8710", "online")
    );
    assert_eq!(
        (devices[2].id.as_str(), devices[2].status.as_str()),
        ("USB-03", "unauthorized")
    );
}

struct FixtureRunner;

struct OversizedRunner;

struct DiagnosticRunner;

struct ConnectedDiagnosticRunner;

struct OfficialVerboseRunner;

struct PhoneCodeRunner;

impl CommandRunner for FixtureRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        if program != "hdc" || args != ["list", "targets", "-v"] {
            return Err(format!("unexpected command: {program} {}", args.join(" ")));
        }
        Ok(CommandOutput {
            success: true,
            stdout: b"USB-01\tConnected\nUSB-02\tOffline\n".to_vec(),
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for OversizedRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: true,
            stdout: vec![b'x'; 17],
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for DiagnosticRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: true,
            stdout: b"daemon unavailable\n[Fail] discovery error\nUSB-01 Connected\n".to_vec(),
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for ConnectedDiagnosticRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: true,
            stdout:
                b"com daemon reported Connected to server\nUSB-01 USB Ready Phone localhost hdc\n"
                    .to_vec(),
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for OfficialVerboseRunner {
    fn output(&self, _program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: true,
            stdout: b"USB-01 USB Ready Phone hdc-1\n192.0.2.1:8710 TCP Connected Tablet hdc-2\nUSB-03 USB Unauthorized Phone hdc-3\n".to_vec(),
            stderr: Vec::new(),
        })
    }
}

impl CommandRunner for PhoneCodeRunner {
    fn output(&self, program: &str, args: &[String]) -> Result<CommandOutput, String> {
        if program != "hdc" {
            return Err(format!("unexpected program: {program}"));
        }
        let stdout = match args {
            [list, targets, verbose]
                if [list.as_str(), targets.as_str(), verbose.as_str()]
                    == ["list", "targets", "-v"] =>
            {
                b"USB-01 USB Ready Phone hdc-1\n".to_vec()
            }
            [target, id, shell, command]
                if target == "-t"
                    && id == "USB-01"
                    && shell == "shell"
                    && command == "ls -1 /version/special_cust" =>
            {
                b"TAS-AL00\n".to_vec()
            }
            _ => return Err(format!("unexpected command: {program} {}", args.join(" "))),
        };
        Ok(CommandOutput {
            success: true,
            stdout,
            stderr: Vec::new(),
        })
    }
}

#[cfg(unix)]
fn slow_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slow-hdc.sh")
}

#[cfg(unix)]
fn oversized_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/oversized-hdc.sh")
}
