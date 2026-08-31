use crate::{ArkLogController, ConnectionState, StreamState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamAction {
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingIntent {
    None,
    Start,
    Stop,
}

pub struct StreamIntent {
    pending: PendingIntent,
}

impl StreamIntent {
    pub fn idle() -> Self {
        Self {
            pending: PendingIntent::None,
        }
    }

    pub fn auto_start() -> Self {
        Self {
            pending: PendingIntent::Start,
        }
    }

    pub fn toggle(&mut self, stream: &StreamState) {
        self.pending = match self.pending {
            PendingIntent::Start => PendingIntent::Stop,
            PendingIntent::Stop => PendingIntent::Start,
            PendingIntent::None if should_stop(stream) => PendingIntent::Stop,
            PendingIntent::None => PendingIntent::Start,
        };
    }

    pub fn next_action(
        &mut self,
        connection: &ConnectionState,
        stream: &StreamState,
    ) -> Option<StreamAction> {
        match self.pending {
            PendingIntent::Start
                if connection == &ConnectionState::Ready && is_inactive(stream) =>
            {
                Some(StreamAction::Start)
            }
            PendingIntent::Start if should_stop(stream) => {
                self.pending = PendingIntent::None;
                None
            }
            PendingIntent::Stop if can_request_stop(stream) => Some(StreamAction::Stop),
            PendingIntent::Stop if is_inactive(stream) || stream == &StreamState::Stopping => {
                self.pending = PendingIntent::None;
                None
            }
            _ => None,
        }
    }

    pub fn complete(&mut self, action: StreamAction) {
        let completed = matches!(
            (self.pending, action),
            (PendingIntent::Start, StreamAction::Start) | (PendingIntent::Stop, StreamAction::Stop)
        );
        if completed {
            self.pending = PendingIntent::None;
        }
    }

    pub fn pending_action(&self) -> Option<StreamAction> {
        match self.pending {
            PendingIntent::None => None,
            PendingIntent::Start => Some(StreamAction::Start),
            PendingIntent::Stop => Some(StreamAction::Stop),
        }
    }

    pub fn reconcile(&mut self, controller: &mut ArkLogController) -> Result<bool, String> {
        let Some(action) =
            self.next_action(controller.connection_state(), controller.stream_state())
        else {
            return Ok(false);
        };
        let result = match action {
            StreamAction::Start => controller.start_stream(),
            StreamAction::Stop => controller.request_stop_stream(),
        };
        self.complete(action);
        result.map(|()| true)
    }
}

fn should_stop(stream: &StreamState) -> bool {
    matches!(
        stream,
        StreamState::Starting | StreamState::Streaming | StreamState::Error { active: true, .. }
    )
}

fn can_request_stop(stream: &StreamState) -> bool {
    matches!(
        stream,
        StreamState::Streaming | StreamState::Error { active: true, .. }
    )
}

fn is_inactive(stream: &StreamState) -> bool {
    matches!(
        stream,
        StreamState::Stopped | StreamState::Error { active: false, .. }
    )
}
