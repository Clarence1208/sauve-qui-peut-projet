extern crate core;

mod decoder;
mod models;
mod player;
mod request_models;
mod server_utils;
mod logger;

use player::start_player_thread;
use request_models::{Message, RegisterTeam};
use server_utils::{parse_token_from_response, receive_message, send_message};
use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc, Mutex, OnceLock};
use std::{env, io, thread};

static SECRET_MAP: OnceLock<Arc<Mutex<HashMap<String, u64>>>> = OnceLock::new();

fn main() -> io::Result<()> {

    // Setup logging
    logger::init_logging("log",&["main", "player", "server_response", "challenge", "hint"])?;


    // Step 1: Get server address from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: worker <server_address>");
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
    let mut team_stream = TcpStream::connect(server_address)?;
    println!("Connected to server at {}", server_address);

    // Initialize the global map
    // fixme try RwLock instead of Mutex as the professor suggested
    SECRET_MAP.set(Arc::new(Mutex::new(HashMap::new()))).unwrap();

    // Step 3: Register the team
    // let team_name = "Team NLCP";
    // fixme random team name generation for testing
    let team_name = format!("Team {}", rand::random::<u32>());

    let register_team_message = Message::RegisterTeam(RegisterTeam {
        name: team_name.to_string(),
    });
    send_message(&mut team_stream, &register_team_message)?;
    println!("Registered team: {}", team_name);

    // Step 4: Receive the registration token
    let response = receive_message(&mut team_stream)?;
    println!("Server response: {}", response);
    println!("Raw server response: {:?}", response);

    eprintln!("Parsing token from response");
    if response.contains("AlreadyRegistered") {
        eprintln!("Team already registered, skipping token parsing");
        return Ok(());
    }

    let registration_token = parse_token_from_response(&response)?;

    // Step 5: Spawn threads for each player
    let players = ["Nino", "Paul", "Loriane"];
    let mut handles = vec![];
    for player in players.iter() {
        let player_name = player.to_string();
        let registration_token = registration_token.clone();
        let server_address = server_address.clone();
        // Spawn a new thread for each player, name the thread with the player's name
        handles.push(
            thread::Builder::new()
                .name(player_name.clone())
                .spawn(move || start_player_thread(player_name, registration_token, server_address))
                .expect("Failed to spawn player thread"),
        );
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Player thread panicked");
    }
    println!("All players have exited the labyrinth. Program completed.");

    // fixme error handling

    Ok(())
}
