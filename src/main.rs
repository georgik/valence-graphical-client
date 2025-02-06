use valence_protocol::block::{PropName, PropValue};
use valence_protocol::packets::play::BlockUpdateS2c;
use bevy::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use bevy::tasks::AsyncComputeTaskPool;
use tokio::sync::mpsc;
use valence_protocol::{PacketDecoder, PacketEncoder, VarInt};
use valence_protocol::decode::PacketFrame;
use valence_protocol::packets::login::{LoginHelloC2s, LoginSuccessS2c, LoginCompressionS2c};
use valence_protocol::packets::play::{GameJoinS2c, KeepAliveS2c, KeepAliveC2s, PlayerPositionLookS2c, PlayerAbilitiesS2c, ChunkDataS2c, ChatMessageS2c, DisconnectS2c, EntityStatusS2c, PlayerListS2c, PlayerRespawnS2c, PlayerSpawnPositionS2c, CommandTreeS2c, UpdateSelectedSlotS2c, AdvancementUpdateS2c, HealthUpdateS2c, EntityAttributesS2c, SynchronizeTagsS2c, ScreenHandlerSlotUpdateS2c, ChatMessageC2s, GameMessageS2c, EntitySetHeadYawS2c, RotateS2c};
use valence_protocol::packets::status::{QueryRequestC2s, QueryResponseS2c};
use valence_protocol::Packet;


#[derive(Resource)]
struct ConnectionStatus {
    message: String,
    connected: bool,
    stream: Option<TcpStream>,
    decoder: Option<PacketDecoder>,
    encoder: Option<PacketEncoder>,
}

#[derive(Resource)]
struct ConnectionEventChannel {
    sender: mpsc::Sender<ApplicationEvent>,
    receiver: mpsc::Receiver<ApplicationEvent>,
}

#[derive(Debug)]
enum ApplicationEvent {
    Connected,
    ChatMessage(String),
    Disconnected(String), // Include a reason for disconnection
}

#[derive(Component)]
struct ConnectionTask;

#[derive(Resource)]
struct ServerAddress(String);

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
        .add_systems(Update, update_connection_status)
        // .add_systems(Update, handle_server_messages)
        .run();
}

fn setup_ui(mut commands: Commands) {
    // Add a 2D camera specifically for UI rendering
    commands.spawn(Camera2dBundle::default());

    commands.spawn(TextBundle {
        text: Text::from_section(
            "Connecting...",
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..default()
            },
        ),
        style: Style {
            align_self: AlignSelf::Center,
            margin: UiRect::all(Val::Auto),
            ..default()
        },
        ..default()
    });
}

async fn connect_and_handle(
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

fn update_connection_status(
    mut connection_status: ResMut<ConnectionStatus>,
    mut query: Query<&mut Text>,
    mut event_receiver: ResMut<ConnectionEventChannel>,
) {
    // Drain the channel of all pending events at once
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
        }
    }

    let mut text = query.single_mut();
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

async fn handle_server_messages_inner(connection_status: &mut ConnectionStatus, sender: mpsc::Sender<ApplicationEvent>) {
    if !connection_status.connected {
        return;
    }

    let mut dec = connection_status.decoder.take().unwrap();
    let mut enc = connection_status.encoder.take().unwrap();
    let mut stream = connection_status.stream.take().unwrap();

    let mut buffer = Vec::with_capacity(4096);
    buffer.resize(4096, 0);

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    println!("Server disconnected.");
                    connection_status.message = "Connection closed.".to_string();
                    connection_status.connected = false;
                    break;
                }
                dec.queue_bytes((&buffer[..size]).into());

                while let Ok(Some(frame)) = dec.try_next_packet() {
                    if let Err(e) = process_packet(frame, &mut dec, &mut enc, &mut stream, sender.clone()).await {
                        println!("Error processing packet or disconnection: {:?}", e);
                        connection_status.message = "Connection closed.".to_string();
                        connection_status.connected = false;
                        break;
                    }
                }
            }
            Err(e) => {
                println!("Error reading from stream: {:?}", e);
                connection_status.message = "Connection closed.".to_string();
                connection_status.connected = false;
                break;
            }
        }
    }

    connection_status.decoder = Some(dec);
    connection_status.encoder = Some(enc);
    connection_status.stream = Some(stream);
}

async fn process_packet(
    frame: PacketFrame,
    dec: &mut PacketDecoder,
    enc: &mut PacketEncoder,
    // socket: &mut TcpSocket<'_>,
    stream: &mut TcpStream,
    sender: mpsc::Sender<ApplicationEvent>,
) -> Result<(), ()> {
    match frame.id {
        LoginCompressionS2c::ID => {
            println!("LoginCompressionS2c");
            // let packet: LoginCompressionS2c = frame.decode().expect("Failed to decode LoginCompressionS2c");
            let threshold = 256;
            // let threshold = packet.threshold.0;
            println!("Compression threshold received: {}", threshold);

            // Set compression threshold for decoder and encoder
            dec.set_compression(valence_protocol::CompressionThreshold(threshold));
            enc.set_compression(valence_protocol::CompressionThreshold(threshold));
        }

        LoginSuccessS2c::ID => {
            // heap_stats();
            // sender.try_send(HardwareEvent::ToggleLed).unwrap();
            let packet: LoginSuccessS2c =
                frame.decode().expect("Failed to decode LoginSuccessS2c");
            println!(
                "Login successful! Username: {}, UUID: {}",
                packet.username, packet.uuid
            );
        }
        GameJoinS2c::ID => {
            // Assuming the player successfully joined the game world.
            println!("GameJoin - skipping deserialization - requires binary compound support");

        }
        PlayerPositionLookS2c::ID => {
            let packet: PlayerPositionLookS2c =
                frame.decode().expect("Failed to decode PlayerPositionLookS2c");
            println!(
                "Player position look: x={}, y={}, z={}, yaw={}, pitch={}",
                packet.position.x, packet.position.y, packet.position.z, packet.yaw, packet.pitch
            );
        }
        KeepAliveS2c::ID => {
            let packet: KeepAliveS2c = frame.decode().expect("Failed to decode KeepAliveS2c");
            println!("KeepAlive received with ID: {}", packet.id);

            // Encode the KeepAliveC2s response
            enc.clear();
            enc.append_packet(&KeepAliveC2s { id: packet.id })
                .expect("Failed to encode KeepAliveC2s");

            let data = enc.take();

            println!("Encoded KeepAliveC2s packet: {:?}", data);

            // Send the packet to the server
            match stream.write_all(&data) {
                Ok(_) => {
                    println!("Successfully sent KeepAliveC2s with ID: {}", packet.id);
                }
                Err(e) => {
                    println!(
                        "Failed to send KeepAliveC2s with ID: {}. Error: {:?}",
                        packet.id, e
                    );
                    return Err(()); // Handle error
                }
            }
            stream.flush().unwrap();
            sender.send(ApplicationEvent::Connected).await.unwrap();
        }
        ChatMessageS2c::ID => {
            let packet: ChatMessageS2c =
                frame.decode().expect("Failed to decode ChatMessageS2c");
            println!("Chat message: {}", packet.message);
        }
        DisconnectS2c::ID => {
            let packet: DisconnectS2c =
                frame.decode().expect("Failed to decode DisconnectS2c");
            println!("Disconnected by server: {}", packet.reason);
            return Err(()); // Exit loop after disconnect
        }
        HealthUpdateS2c::ID => {
            let packet: HealthUpdateS2c =
                frame.decode().expect("Failed to decode HealthUpdateS2c");
            println!(
                "Health Update: health={}, saturation={}",
                packet.health, packet.food_saturation
            );
        }
        ChunkDataS2c::ID => {
            println!("Received chunk data.");
        }
        PlayerSpawnPositionS2c::ID => {
            // let packet: PlayerSpawnPositionS2c =
            //     frame.decode().expect("Failed to decode PlayerSpawnPositionS2c");
            // println!(
            //     "Player spawn position: x={}, y={}, z={}",
            //     packet.position.x, packet.position.y, packet.position.z
            // );
            println!("PlayerSpawnPositionS2c");
        }
        PlayerAbilitiesS2c::ID => {
            // heap_stats();
            let packet: PlayerAbilitiesS2c =
                frame.decode().expect("Failed to decode PlayerAbilitiesS2c");
            println!("Player abilities: {:?}", packet.flags);
        }
        EntityStatusS2c::ID => {
            let packet: EntityStatusS2c =
                frame.decode().expect("Failed to decode EntityStatusS2c");
            println!("Entity status: entity_id={}, status={}", packet.entity_id, packet.entity_status);
        }
        EntityAttributesS2c::ID => {
            let packet: EntityAttributesS2c =
                frame.decode().expect("Failed to decode EntityAttributesS2c");
            println!("Entity attributes: entity_id={:?}, attributes={:?}", packet.entity_id, packet.properties);
        }
        UpdateSelectedSlotS2c::ID => {
            let packet: UpdateSelectedSlotS2c =
                frame.decode().expect("Failed to decode UpdateSelectedSlotS2c");
            println!("Selected slot updated: slot={}", packet.slot);
        }
        PlayerListS2c::ID => {
            let packet: PlayerListS2c =
                frame.decode().expect("Failed to decode PlayerListS2c");
            println!("Player list: {:?}", packet.entries);
        }
        ScreenHandlerSlotUpdateS2c::ID => {
            println!("Received ScreenHandlerSlotUpdateS2c.");
        }
        AdvancementUpdateS2c::ID => {
            let packet: AdvancementUpdateS2c =
                frame.decode().expect("Failed to decode AdvancementUpdateS2c");
            println!("Advancement update: {:?}", packet.identifiers);
        }
        CommandTreeS2c::ID => {
            println!("Received CommandTreeS2c.");
        }
        SynchronizeTagsS2c::ID => {
            println!("Received SynchronizeTagsS2c.");
        }
        GameMessageS2c::ID => {
            let packet: GameMessageS2c =
                frame.decode().expect("Failed to decode GameMessageS2c");
            let received_message = packet.chat.to_string();
            println!("Received message: {:?}", received_message);

            if received_message.contains("How are you?") {
                // Send a chat message "ahoj"
                let message = ChatMessageC2s {
                    message: valence_protocol::Bounded("I feel good. I'm running at 240 MHz.".into()), // The message content
                    timestamp: 0,
                    salt: 0,
                    signature: None,
                    message_count: Default::default(),
                    acknowledgement: Default::default(),
                };

                enc.clear();
                enc.append_packet(&message)
                    .expect("Failed to encode ChatMessageC2s");
                let data = enc.take();

                println!("Sending ChatMessageC2s packet: {:?}", data);

                match stream.write_all(&data) {
                    Ok(_) => {
                        println!("Chat message sent: 'ahoj'");
                    }
                    Err(e) => {
                        println!("Failed to send chat message. Error: {:?}", e);
                    }
                }
                stream.flush().unwrap();
            }
        }
        EntitySetHeadYawS2c::ID => {
            println!("EntitySetHeadYawS2c");
        }
        RotateS2c::ID => {
            println!("RotateS2c");
        }
        BlockUpdateS2c::ID => {
            println!("BlockUpdateS2c");

            // Attempt to decode the packet
            let packet: BlockUpdateS2c = match frame.decode() {
                Ok(decoded_packet) => decoded_packet,
                Err(err) => {
                    println!("Failed to decode BlockUpdateS2c: {:?}", err);
                    return Err(()); // Skip further processing for this packet
                }
            };

            // Safely get the "Lit" property and handle potential absence
            if let Some(PropValue::True) = packet.block_id.get(PropName::Lit) {
                println!("Block is lit, turning on LED.");
                sender.send(ApplicationEvent::ChatMessage("Led - on.".to_string())).await.unwrap();
            } else {
                println!("Block is not lit, turning off LED.");
                sender.send(ApplicationEvent::ChatMessage("Led - off.".to_string())).await.unwrap();
            }
        }

        _ => println!("Unhandled packet ID: 0x{:X}", frame.id),
    }
    // heap_stats();
    Ok(())

}