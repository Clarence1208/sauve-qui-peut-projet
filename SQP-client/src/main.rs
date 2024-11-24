use serde::{Deserialize, Serialize};
use serde_json;
use std::{env, thread};
use std::io::{self, Read, Write};
use std::net::TcpStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Direction {
    Front,
    Back,
    Left,
    Right,
}


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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Action {
    MoveTo: Direction,
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
    Action(Action),
}

/**
 * Send a message to the server
 *
 * @param stream: &mut TcpStream - The TCP stream to send the message
 * @param message: &Message - The message to send
 * @return io::Result<()> - The result of the operation
 */
fn send_message(stream: &mut TcpStream, message: &impl Serialize) -> io::Result<()> {
    // Log the preparation step
    println!("Preparing to send message...");

    // Serialize the message to JSON
    let serialized_message = serde_json::to_string(&message).expect("Failed to serialize message");
    println!("Serialized message: {}", serialized_message);

    // Send the message length (u32 in little-endian)
    let message_length = serialized_message.len() as u32;
    stream.write_all(&message_length.to_le_bytes()).map_err(|e| {
        eprintln!("Failed to send message length: {}", e);
        e
    })?;
    println!("Sent message length: {}", message_length);

    // Send the JSON message
    stream.write_all(serialized_message.as_bytes()).map_err(|e| {
        eprintln!("Failed to send message payload: {}", e);
        e
    })?;
    println!("Message sent successfully: {}", serialized_message);

    Ok(())
}


fn receive_message(stream: &mut TcpStream) -> io::Result<String> {
    // Read the length of the incoming message
    let mut length_buffer = [0; 4];
    stream.read_exact(&mut length_buffer).map_err(|_| {
        io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read message length")
    })?;
    let message_length = u32::from_le_bytes(length_buffer) as usize;
    println!("Received message length: {}", message_length);

    let mut message_buffer = Vec::with_capacity(message_length);
    let mut total_read = 0;

    while total_read < message_length {
        let mut chunk = vec![0; message_length - total_read];
        let bytes_read = stream.read(&mut chunk)?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Connection closed before full message was received",
            ));
        }
        total_read += bytes_read;
        message_buffer.extend_from_slice(&chunk[..bytes_read]);
    }

    String::from_utf8(message_buffer)
        .map(|msg| msg.trim_matches(char::from(0)).to_string())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 message received"))
}


fn parse_token_from_response(response: &str) -> io::Result<String> {
    let registration_result: serde_json::Value =
        serde_json::from_str(response).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to parse server response"))?;

    registration_result["RegisterTeamResult"]["Ok"]["registration_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Registration token not found"))
}

/**
 * The player_thread function represents the main logic for each player thread.
 * It subscribes the player to the server, then enters a loop to solve the labyrinth.
 *
 * @param player_name: String - The name of the player
 * @param registration_token: String - The registration token for the player
 * @param server_address: String - The address of the server
 */
fn player_thread(player_name: String, registration_token: String, server_address: String) {
    let mut player_stream = TcpStream::connect(server_address).expect("Failed to connect to server");
    println!("Connected for player: {}", player_name);

    // Subscribe the player
    let subscribe_player_message = Message::SubscribePlayer(SubscribePlayer {
        name: player_name.clone(),
        registration_token: registration_token.clone(),
    });
    send_message(&mut player_stream, &subscribe_player_message).expect("Failed to subscribe player");
    println!("Subscribed player: {}", player_name);

    let response = receive_message(&mut player_stream).expect("Failed to receive subscription response");
    println!("Server response for player {}: {}", player_name, response);

    // timeout 1/100 of a second
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Labyrinth-solving loop
    let mut current_direction = Direction::Front; // Start by trying to move forward

    loop {
        // Send the current movement action
        let action_message = Message::Action(Action {
            MoveTo: current_direction.clone(),
        });
        send_message(&mut player_stream, &action_message).expect("Failed to send action");
        println!("Player {} sent action: {:?}", player_name, current_direction);

        // Receive the server's response to the action
        let action_response = receive_message(&mut player_stream).expect("Failed to receive action response");
        println!("Player {} received response: {}", player_name, action_response);

        // Check for exit condition
        if action_response.contains("FoundExit") {
            println!("Player {} found the exit!", player_name);
            break; // Exit the loop
        }

        // Check if movement was blocked
        if action_response.contains("CannotPassThroughWall") {
            // If movement is blocked, turn right
            current_direction = match current_direction {
                Direction::Front => Direction::Right,
                Direction::Right => Direction::Back,
                Direction::Back => Direction::Left,
                Direction::Left => Direction::Front,
            };
            println!("Player {} hit a wall, turning to {:?}", player_name, current_direction);
        }
    }
}

fn main() -> io::Result<()> {
    // Step 1: Get server address from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: worker <server_address>");use serde::{Deserialize, Serialize};
        std::process::exit(1);
    }
    let server_address = &args[1];

    // Validate the address format
    if !server_address.contains(':') {
        eprintln!("Error: Invalid server address. Use <host:port> format (e.g., 127.0.0.1:8778).");
        std::process::exit(1);
    }

    // Step 2: Connect to the server
    let mut teamStream = TcpStream::connect(server_address)?;
    println!("Connected to server at {}", server_address);

    // Step 3: Register the team
    // let team_name = "Team NLCP";
    // fixme random team name generation for testing
    let team_name = format!("Team {}", rand::random::<u32>());

    let register_team_message = Message::RegisterTeam(RegisterTeam {
        name: team_name.to_string(),
    });
    send_message(&mut teamStream, &register_team_message)?;
    println!("Registered team: {}", team_name);


    // Step 4: Receive the registration token
    let response = receive_message(&mut teamStream)?;
    println!("Server response: {}", response);
    println!("Raw server response: {:?}", response);


    eprintln!("Parsing token from response");
    if response.contains("AlreadyRegistered") {
        eprintln!("Team already registered, skipping token parsing");
        return Ok(());
    }

    let registration_token = parse_token_from_response(&response)?;


    // Step 5: Spawn threads for each player
    let players = ["Nino", "Paul", "Lorianne", "Clarence"];
    let mut handles = vec![];
    for player in players.iter() {
        let player_name = player.to_string();
        let registration_token = registration_token.clone();
        let server_address = server_address.clone();
        handles.push(thread::spawn(move || {
            player_thread(player_name, registration_token, server_address);
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Player thread panicked");
    }
    println!("All players have exited the labyrinth. Program completed.");


    /*
    // Step 5: Register players
    let players = ["Nino", "Paul", "Lorianne", "Clarence"];
    for player in players {
        //fixme it is a workaround because i can't keep the connection alive from here on
        let mut stream = TcpStream::connect(server_address)?;
        println!("Reconnected to server for player registration");

        let subscribe_player_message = Message::SubscribePlayer(SubscribePlayer {
            name: player.to_string(),
            registration_token: registration_token.clone(),
        });
        send_message(&mut stream, &subscribe_player_message)?;
        println!("Trying to register player: {}", player);

        let response = receive_message(&mut stream)?;
        println!("Server response: {}", response);
    }

    // Step 6: Send actions
    let actions = [
        Action { MoveTo: Direction::Front },
        Action { MoveTo: Direction::Back },
        Action { MoveTo: Direction::Left },
        Action { MoveTo: Direction::Right },
    ];
    for action in actions.iter() {
        //fixme it is a workaround because i can't keep the connection alive from here on
        let mut stream = TcpStream::connect(server_address)?;
        // Dereference action to pass the value
        let action_message = Message::Action(action.clone());
        send_message(&mut stream, &action_message)?;
        println!("Sent action: {:?}", action);
    }
*/

    // fixme error handling

    Ok(())
}
