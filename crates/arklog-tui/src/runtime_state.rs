#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesiredStream {
    Running,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamAction {
    Start,
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Refreshing,
    Ready,
    SelectionRequired,
    NoDevices,
    Error(String),
}

impl ConnectionState {
    pub fn message(&self) -> &str {
        match self {
            Self::Refreshing => "Refreshing devices",
            Self::Ready => "Devices ready",
            Self::SelectionRequired => "Select an online device",
            Self::NoDevices => "No devices",
            Self::Error(message) => message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamState {
    Stopped,
    Starting,
    Streaming,
    Stopping,
    Error { message: String, active: bool },
}

impl StreamState {
    pub fn message(&self) -> &str {
        match self {
            Self::Stopped => "Stopped",
            Self::Starting => "Starting",
            Self::Streaming => "Streaming",
            Self::Stopping => "Stopping",
            Self::Error { message, .. } => message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaultLogState {
    Idle,
    Refreshing,
    Ready,
    Error(String),
}

impl FaultLogState {
    pub fn message(&self) -> &str {
        match self {
            Self::Idle => "Not loaded",
            Self::Refreshing => "Refreshing fault logs",
            Self::Ready => "Fault logs ready",
            Self::Error(message) => message,
        }
    }
}
