use std::borrow::Cow;
use std::io::Write;
use std::net::TcpStream;
use tokio::sync::mpsc;
use valence_protocol::decode::PacketFrame;
use valence_protocol::{PacketDecoder, PacketEncoder};
use valence_protocol::block::{PropName, PropValue};
use valence_protocol::packets::login::{LoginCompressionS2c, LoginSuccessS2c};
use valence_protocol::packets::play::{AdvancementUpdateS2c, BlockUpdateS2c, ChatMessageC2s, ChatMessageS2c, ChunkDataS2c, CommandTreeS2c, DisconnectS2c, EntityAttributesS2c, EntitySetHeadYawS2c, EntityStatusS2c, GameJoinS2c, GameMessageS2c, HealthUpdateS2c, KeepAliveC2s, KeepAliveS2c, PlayerAbilitiesS2c, PlayerListS2c, PlayerPositionLookS2c, PlayerSpawnPositionS2c, RotateS2c, ScreenHandlerSlotUpdateS2c, SynchronizeTagsS2c, UpdateSelectedSlotS2c};
use crate::events::{ApplicationEvent, ChunkBlockData};
use crate::connection::ConnectionStatus;
use std::io::Read;
use valence_protocol::Packet;
use crate::events::ApplicationEvent::ChunkData;

pub(crate) async fn handle_server_messages_inner(connection_status: &mut ConnectionStatus, sender: mpsc::Sender<ApplicationEvent>) {
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
            println!("GameJoinS2c");
            let packet: GameJoinS2c = frame.decode().expect("Failed to decode GameJoinS2c");

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
            let packet: ChunkDataS2c = frame.decode().expect("Failed to decode ChunkDataS2c");

            println!("Chunk data received");
            println!("Position: x={}, z={}, count={}", packet.pos.x, packet.pos.z, packet.blocks_and_biomes.len());
            let data = ChunkBlockData {
                pos: valence_protocol::ChunkPos { x: packet.pos.x, z: packet.pos.z },
                blocks: packet.blocks_and_biomes.to_vec(),
            };

            sender.send(ApplicationEvent::ChunkData(data)).await.unwrap();
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
            let packet: SynchronizeTagsS2c =
                frame.decode().expect("Failed to decode SynchronizeTagsS2c");
            // println!("Tags: {:?}", packet.groups);
        }
        GameMessageS2c::ID => {
            let packet: GameMessageS2c =
                frame.decode().expect("Failed to decode GameMessageS2c");
            let received_message = packet.chat.to_string();
            println!("Received message: {:?}", received_message);

            if received_message.contains("How are you?") {
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
                sender.send(ApplicationEvent::LampOn).await.unwrap();
            } else {
                println!("Block is not lit, turning off LED.");
                sender.send(ApplicationEvent::LampOff).await.unwrap();
            }
        }

        _ => println!("Unhandled packet ID: 0x{:X}", frame.id),
    }
    // heap_stats();
    Ok(())

}