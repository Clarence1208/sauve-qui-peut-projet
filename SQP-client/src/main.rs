use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;

/**
 * The RegisterTeam struct represents the content of the RegisterTeam message.
 * It contains the team name.
 */

#[derive(Serialize, Deserialize, Debug)]
struct RegisterTeam {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubscribePlayer {
    name: String,
    registration_token: String,
}

/**
 * The message enum represents the different types of messages that can be sent to the server.
 * Each message type is represented by a struct.
 */
#[derive(Serialize, Deserialize, Debug)]
enum Message {
    #[serde(rename_all = "camelCase")]
    RegisterTeam(RegisterTeam),
    SubscribePlayer(SubscribePlayer),
}

/**
 * Send a message to the server
 *
 * @param stream: &mut TcpStream - The TCP stream to send the message
 * @param message: &Message - The message to send
 * @return io::Result<()> - The result of the operation
 */
fn send_message(stream: &mut TcpStream, message: &impl Serialize) -> io::Result<()> {
    // Serialize the message to JSON
    let serialized_message = serde_json::to_string(&message).expect("Failed to serialize message");
    println!("Sending message: {}", serialized_message);

    // Send the message length
    let message_length = serialized_message.len() as u32;
    stream.write_all(&message_length.to_le_bytes())?;

    // Send the JSON message
    stream.write_all(serialized_message.as_bytes())?;
    Ok(())
}

fn receive_message(stream: &mut TcpStream) -> io::Result<String> {
    let mut length_buffer = [0; 4];
    stream.read_exact(&mut length_buffer)?;
    let message_length = u32::from_le_bytes(length_buffer) as usize;

    let mut message_buffer = vec![0; message_length];
    stream.read_exact(&mut message_buffer)?;

    // Handle responses with embedded unexpected bytes
    if let Ok(clean_message) = String::from_utf8(message_buffer.clone()) {
        return Ok(clean_message.trim_matches(char::from(0)).to_string());
        return Ok(clean_message.trim_matches(char::from(0)).to_string());
    }
    // Read the message length
    let mut length_buffer = [0; 4];
    stream.read_exact(&mut length_buffer)?;
    let message_length = u32::from_le_bytes(length_buffer) as usize;

    // Read the actual message
    let mut message_buffer = vec![0; message_length];
    stream.read_exact(&mut message_buffer)?;

    // Convert the message to a String
    let message = String::from_utf8(message_buffer).expect("Failed to parse message as UTF-8");
    Ok(message)
}

fn parse_token_from_response(response: &str) -> io::Result<String> {
    // Check if the server provided a registration token
    if response.trim() == "2" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Server did not provide a registration token",
        ));
    }

    // Parse as JSON
    let registration_result: serde_json::Value =
        serde_json::from_str(response).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse server response"))?;

    let token = registration_result["RegisterTeamResult"]["Ok"]["registration_token"]
        .as_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to extract registration token"))?;
    Ok(token.to_string())
}

fn main() -> io::Result<()> {
    // Step 1: Get server address from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: worker <server_address>");
        std::process::exit(1);
    }
    let server_address = &args[1];

    // Validate the address format
    if !server_address.contains(':') {
        eprintln!("Error: Invalid server address. Use <host:port> format (e.g., 127.0.0.1:8778).");
        std::process::exit(1);
    }

    // Step 2: Connect to the server
    let mut stream = TcpStream::connect(server_address)?;
    println!("Connected to server at {}", server_address);

    // Step 3: Register the team
    let team_name = "Team NLCP";
    let register_team_message = Message::RegisterTeam(RegisterTeam {
        name: team_name.to_string(),
    });
    send_message(&mut stream, &register_team_message)?;
    println!("Registered team: {}", team_name);


    // Step 4: Receive the registration token
    let response = receive_message(&mut stream)?;
    println!("Server response: {}", response);
    println!("Raw server response: {:?}", response);


    eprintln!("Parsing token from response");
    if response.contains("AlreadyRegistered") {
        eprintln!("Team already registered, skipping token parsing");
        return Ok(());
    }

    let registration_token = parse_token_from_response(&response)?;

    // Step 5: Register players
    let players = ["Nino", "Paul", "Lorianne", "Clarence"];
    for player in players {
        let subscribe_player_message = Message::SubscribePlayer(SubscribePlayer {
            name: player.to_string(),
            registration_token: registration_token.to_string(),
        });
        send_message(&mut stream, &subscribe_player_message)?;
        println!("Registered player: {}", player);
    }


    // TODO: Handle server responses
    // TODO: next step: register players
    // fixme error handling

    Ok(())
}
