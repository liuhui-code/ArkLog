use arklog::{ConnectionState, StreamAction, StreamIntent, StreamState};

#[test]
fn ctrl_s_during_initial_refresh_cancels_the_pending_auto_start() {
    let mut intent = StreamIntent::auto_start();

    assert_eq!(
        intent.next_action(&ConnectionState::Refreshing, &StreamState::Stopped),
        None
    );
    intent.toggle(&StreamState::Stopped);

    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Stopped),
        None
    );
}

#[test]
fn ctrl_s_while_stopping_queues_one_restart_after_stop_finishes() {
    let mut intent = StreamIntent::idle();

    intent.toggle(&StreamState::Streaming);
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Streaming),
        Some(StreamAction::Stop)
    );
    intent.complete(StreamAction::Stop);

    intent.toggle(&StreamState::Stopping);
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Stopping),
        None
    );
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Stopped),
        Some(StreamAction::Start)
    );
    intent.complete(StreamAction::Start);
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Streaming),
        None
    );
}

#[test]
fn two_ctrl_s_presses_while_stopping_cancel_the_queued_restart() {
    let mut intent = StreamIntent::idle();

    intent.toggle(&StreamState::Stopping);
    intent.toggle(&StreamState::Stopping);

    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Stopping),
        None
    );
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Stopped),
        None
    );
}

#[test]
fn ctrl_s_while_starting_stops_as_soon_as_the_stream_becomes_active() {
    let mut intent = StreamIntent::idle();

    intent.toggle(&StreamState::Starting);
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Starting),
        None
    );
    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &StreamState::Streaming),
        Some(StreamAction::Stop)
    );
}

#[test]
fn queued_restart_is_satisfied_when_stop_fails_and_the_old_stream_stays_active() {
    let mut intent = StreamIntent::idle();
    intent.toggle(&StreamState::Stopping);
    let active_error = StreamState::Error {
        message: "Failed to stop HDC".to_string(),
        active: true,
    };

    assert_eq!(
        intent.next_action(&ConnectionState::Ready, &active_error),
        None
    );
    assert_eq!(
        intent.next_action(
            &ConnectionState::Ready,
            &StreamState::Error {
                message: "HDC later exited".to_string(),
                active: false,
            },
        ),
        None
    );
}
