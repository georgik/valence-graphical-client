use valence_protocol::packets::play::ChunkDataS2c;

#[derive(Clone, Debug)]
pub struct ChunkBlockData {
    pub pos: valence_protocol::ChunkPos,
    pub blocks: Vec<u8>,
}


#[derive(Debug)]
pub(crate) enum ApplicationEvent {
    Connected,
    LampOn,
    LampOff,
    ChatMessage(String),
    Disconnected(String),
    ChunkData(ChunkBlockData),
}
