
#[derive(Debug)]
pub(crate) enum ApplicationEvent {
    Connected,
    LampOn,
    LampOff,
    ChatMessage(String),
    Disconnected(String), // Include a reason for disconnection
}
