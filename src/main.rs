mod connection;
mod networking;
mod events;
mod rendering;
mod world;

use valence_protocol::block::{PropName, PropValue};
use valence_protocol::packets::play::BlockUpdateS2c;
use bevy::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use bevy::render::view::NoFrustumCulling;
use bevy::tasks::AsyncComputeTaskPool;
use tokio::sync::mpsc;
use valence_protocol::{PacketDecoder, PacketEncoder, VarInt};
use valence_protocol::Packet;

use connection::{connect_and_handle, ConnectionStatus};
use events::ApplicationEvent;
use crate::rendering::setup_ui;


#[derive(Resource)]
struct ConnectionEventChannel {
    sender: mpsc::Sender<ApplicationEvent>,
    receiver: mpsc::Receiver<ApplicationEvent>,
}

#[derive(Component)]
struct ConnectionTask;

#[derive(Resource)]
struct ServerAddress(String);

#[derive(Component)]
struct WorldCube {
    x: usize,
    y: usize,
    z: usize,
}


#[derive(Component)]
struct GlowingCube;

fn main() {
    // env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    // App::new().add_systems(Startup, connect_to_server).run();
    let (sender, receiver) = mpsc::channel(32);
    App::new()
        .insert_resource(ServerAddress("127.0.0.1:25565".to_string()))
        .insert_resource(ConnectionStatus {
            message: "Connecting...".to_string(),
            connected: false,
            stream: None,
            decoder: None,
            encoder: None,
        })
        .insert_resource(ConnectionEventChannel {
            sender,
            receiver,
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_ui)
        .add_systems(Startup, start_connection_task)
        .add_systems(Update, process_application_event)
        .run();
}





#[derive(Component)]
struct WorldMesh;


fn start_connection_task(
    mut commands: Commands,
    event_sender: Res<ConnectionEventChannel>,
    server_address: Res<ServerAddress>,
) {
    println!("Starting connection task...");
    let sender = event_sender.sender.clone();
    let address = server_address.0.clone();

    commands.spawn(ConnectionTask); // You can still spawn an entity if needed

    let async_compute_task_pool = AsyncComputeTaskPool::get();

    async_compute_task_pool.spawn(async move {
        connect_and_handle(sender, address).await;
    }).detach();

}

fn process_application_event(
    mut connection_status: ResMut<ConnectionStatus>,
    mut text_query: Query<&mut Text>, // Renamed for clarity
    mut event_receiver: ResMut<ConnectionEventChannel>,
    mut material_query: Query<&mut Handle<StandardMaterial>, With<GlowingCube>>, // Query for material
    mut materials: ResMut<Assets<StandardMaterial>>, // Access to materials
) {
    while let Ok(event) = event_receiver.receiver.try_recv() {
        println!("Received event: {:?}", event);
        match event {
            ApplicationEvent::Connected => {
                connection_status.message = "Connected!".to_string();
                connection_status.connected = true;
            }
            ApplicationEvent::ChatMessage(message) => {
                println!("** Received ChatMessage: {}", message);
                connection_status.message = message;
            }
            ApplicationEvent::Disconnected(reason) => {
                connection_status.message = format!("Connection failed: {}", reason);
                connection_status.connected = false;
                connection_status.stream = None;
                connection_status.decoder = None;
                connection_status.encoder = None;
            }
            ApplicationEvent::LampOn => {
                if let Ok(mut material_handle) = material_query.get_single_mut() {
                    let mut material = materials.get_mut(&*material_handle).unwrap();
                    material.base_color = Color::rgb(1.0, 1.0, 0.0);
                }
            }
            ApplicationEvent::LampOff => {
                if let Ok(mut material_handle) = material_query.get_single_mut() {
                    let mut material = materials.get_mut(&*material_handle).unwrap();
                    material.base_color = Color::rgb(0.0, 1.0, 0.0);
                }
            }
        }
    }

    let mut text = text_query.single_mut(); // Use renamed query
    if text.sections[0].value != connection_status.message {
        println!("Connection status message updated: {}", connection_status.message);
        text.sections[0].value = connection_status.message.clone();
    }
}
fn connect_to_server(connection_status: &mut ConnectionStatus) {
    let server_address = "127.0.0.1:25565";

    match TcpStream::connect(server_address) {
        Ok(stream) => {
            println!("Successfully connected to server at {}", server_address);
            connection_status.message = "Connected!".to_string();
            connection_status.connected = true;
            connection_status.stream = Some(stream.try_clone().unwrap()); // Clone the stream
            connection_status.decoder = Some(PacketDecoder::new());
            connection_status.encoder = Some(PacketEncoder::new());

            // Perform handshake and login here (same as before)
            let mut enc = connection_status.encoder.take().unwrap();

            // Handshake
            let next_state = valence_protocol::packets::handshaking::handshake_c2s::HandshakeNextState::Login;
            let handshake_packet = valence_protocol::packets::handshaking::handshake_c2s::HandshakeC2s {
                protocol_version: VarInt(763),
                server_address: valence_protocol::Bounded("127.0.0.1"),
                server_port: 25566,
                next_state,
            };

            enc.append_packet(&handshake_packet)
                .expect("Failed to encode handshake packet");
            connection_status.stream.as_mut().unwrap()
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
            connection_status.stream.as_mut().unwrap()
                .write_all(&enc.take())
                .expect("Failed to send handshake packet");

            connection_status.encoder = Some(enc);
        }
        Err(e) => {
            println!("Failed to connect to server at {}: {}", server_address, e);
            connection_status.message = format!("Connection failed: {}", e);
            connection_status.connected = false;
        }
    }
}

