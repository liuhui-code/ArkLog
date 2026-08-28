use arklog_core::{CommandOutput, CommandRunner, HdcClient};

#[test]
fn lists_structured_devices_from_hdc_targets() {
    let client = HdcClient::with_runner("hdc", FixtureRunner);

    let devices = client.list_devices().expect("devices");

    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].id, "USB-01");
    assert_eq!(devices[0].status, "online");
    assert_eq!(devices[1].id, "USB-02");
    assert_eq!(devices[1].status, "offline");
}

struct FixtureRunner;

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
