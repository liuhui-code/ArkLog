use arklog::ActionStatus;

#[test]
fn successful_background_poll_does_not_erase_the_last_action_error() {
    let mut status = ActionStatus::new(Some("Connect server failed".to_string()));

    status.record_background(Ok(()));
    assert_eq!(status.message_or("No devices"), "Connect server failed");

    status.record_action(Ok(()));
    assert_eq!(status.message_or("Ready"), "Ready");
}
