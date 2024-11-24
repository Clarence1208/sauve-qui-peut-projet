use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::io::{self, Write};
use std::net::TcpStream;

/**
 * The RegisterTeam struct represents the content of the RegisterTeam message.
 * It contains the team name.
 */
#[derive(Serialize, Deserialize, Debug)]
struct RegisterTeam {
    name: String,
}

/**
 * The message enum represents the different types of messages that can be sent to the server.
 * Each message type is represented by a struct.
 */
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "content")]
enum Message {
    RegisterTeam(RegisterTeam),
}

/**
 * Send a message to the server
 *
 * @param stream: &mut TcpStream - The TCP stream to send the message
 * @param message: &Message - The message to send
 * @return io::Result<()> - The result of the operation
 */
fn send_message(stream: &mut TcpStream, message: &Message) -> io::Result<()> {
    // Serialize the message to JSON
    let serialized_message = serde_json::to_string(&message).expect("Failed to serialize message");

    // Send the message length (u32)
    let message_length = serialized_message.len() as u32;
    stream.write_all(&message_length.to_le_bytes())?;

    // Send the JSON message
    stream.write_all(serialized_message.as_bytes())?;
    Ok(())
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

    // Step 4: Serialize the message to JSON
    let serialized_message = serde_json::to_string(&register_team_message).expect("Failed to serialize message");

    // Step 5: Send the message length (u32 in little-endian)
    let message_length = serialized_message.len() as u32;
    stream.write_all(&message_length.to_le_bytes())?;

    // Step 6: Send the JSON message
    stream.write_all(serialized_message.as_bytes())?;
    println!("Registered team: {}", team_name);

    // TODO: Handle server responses
    // TODO: next step: register players
    // fixme error handling

    Ok(())
}
