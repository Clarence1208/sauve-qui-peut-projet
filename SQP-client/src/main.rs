extern crate core;

mod player;
mod models;
mod server_utils;
mod decoder;

use models::{Action, Direction, Message, RegisterTeam, SubscribePlayer};
use player::player_thread;
use server_utils::{parse_token_from_response, receive_message, send_message};
use std::net::TcpStream;
use std::{env, io, thread};


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
    let mut team_stream = TcpStream::connect(server_address)?;
    println!("Connected to server at {}", server_address);

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
    // let players = ["Nino", "Paul", "Lorianne", "Clarence"];
    // fixme only 1 player for testing
    let players = ["Nino"];
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

    // fixme error handling

    Ok(())
}
