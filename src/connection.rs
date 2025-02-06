use bevy::prelude::Resource;
use tokio::sync::mpsc;
use valence_protocol::{PacketDecoder, PacketEncoder, VarInt};
use crate::events::ApplicationEvent;
use crate::networking::handle_server_messages_inner;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Resource)]
pub struct ConnectionStatus {
    pub(crate) message: String,
    pub(crate) connected: bool,
    pub(crate) stream: Option<TcpStream>,
    pub(crate) decoder: Option<PacketDecoder>,
    pub(crate) encoder: Option<PacketEncoder>,
}

pub(crate) async fn connect_and_handle(
    sender: mpsc::Sender<ApplicationEvent>,
    server_address: String,
) {
    match TcpStream::connect(&server_address) {
        Ok(stream) => {
            println!("Successfully connected to server at {}", server_address);

            let _ = sender.send(ApplicationEvent::Connected).await;

            let mut connection_status = ConnectionStatus {
                message: String::new(),
                connected: true,
                stream: Some(stream), // Store the connected stream
                decoder: Some(PacketDecoder::new()),
                encoder: Some(PacketEncoder::new()),
            };

            connect_to_server_inner(&mut connection_status); // Perform handshake/login

            handle_server_messages_inner(&mut connection_status, sender.clone()).await;

            let _ = sender.send(ApplicationEvent::Disconnected("Connection closed".to_string())).await; // Signal disconnection
        }
        Err(e) => {
            println!("Failed to connect to server at {}: {}", server_address, e);
            let _ = sender.send(ApplicationEvent::Disconnected(e.to_string())).await; // Signal disconnection with error
        }
    }
}


fn connect_to_server_inner(connection_status: &mut ConnectionStatus) {

    let mut enc = connection_status.encoder.take().unwrap(); // Take encoder

    // Handshake
    let next_state = valence_protocol::packets::handshaking::handshake_c2s::HandshakeNextState::Login;
    let handshake_packet = valence_protocol::packets::handshaking::handshake_c2s::HandshakeC2s {
        protocol_version: VarInt(763),
        server_address: valence_protocol::Bounded("127.0.0.1"), // Or use server_address variable
        server_port: 25566,
        next_state,
    };

    enc.append_packet(&handshake_packet)
        .expect("Failed to encode handshake packet");
    connection_status.stream.as_mut().unwrap() // Use the existing stream
        .write_all(&enc.take())
        .expect("Failed to send handshake packet");

    // Login
    let login_start_packet =
        valence_protocol::packets::login::login_hello_c2s::LoginHelloC2s {
            username: valence_protocol::Bounded("ESP32-S3"), // Replace with your username
            profile_id: None,                                // Optional in offline mode
        };

    enc.append_packet(&login_start_packet)
        .expect("Failed to encode LoginHelloC2s packet");
    connection_status.stream.as_mut().unwrap() // Use the existing stream
        .write_all(&enc.take())
        .expect("Failed to send handshake packet");

    connection_status.encoder = Some(enc); // Put encoder back
}
