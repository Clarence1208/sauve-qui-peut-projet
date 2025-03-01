use clap::{App, Arg, SubCommand};
use log::{debug, error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use SQP_common::error::{Error as SqpError, Error};
use SQP_common::server_utils::{parse_token_from_response, receive_message, send_message};

mod maze_generator;
use maze_generator::generate_maze;

mod encoder;
use encoder::encode;

mod server_request_models;
use crate::server_request_models::Direction;
use server_request_models::{Action, Message, RegisterTeam, SubscribePlayer};
use SQP_common::error::NetworkError::SendPayloadFailed;
use SQP_common::logger;

struct Labyrinth {
    width: usize,
    height: usize,
    cells: Vec<Vec<Cell>>,
    exit_position: (usize, usize),
}

#[derive(Clone)]
struct Cell {
    north_wall: bool,
    east_wall: bool,
    south_wall: bool,
    west_wall: bool,
    has_hint: bool,
    has_exit: bool,
}

struct Player {
    id: usize,
    name: String,
    team_name: String,
    position: (usize, usize),
    direction: MapDirection,
    moves: usize,
}

struct Team {
    name: String,
    registration_token: String,
    expected_players: usize,
    players: Vec<String>,
}

struct ServerState {
    teams: HashMap<String, Team>,
    players: HashMap<String, Player>,
    labyrinth: Labyrinth,
    next_player_id: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
enum MapDirection {
    North,
    South,
    East,
    West,
}

// Message types from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename_all = "camelCase")]
    RegisterTeam { name: String },
    #[serde(rename_all = "camelCase")]
    SubscribePlayer {
        name: String,
        registration_token: String,
    },
    #[serde(rename_all = "camelCase")]
    Action { action: PlayerAction },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PlayerAction {
    #[serde(rename_all = "camelCase")]
    MoveTo(Direction),
    #[serde(rename_all = "camelCase")]
    SolveChallenge { answer: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum RegisterTeamResult {
    Ok {
        expected_players: usize,
        registration_token: String,
    },
    Error(String),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SubscribePlayerResult {
    Ok,
    Error(String),
}

#[derive(Debug, Serialize)]
struct RelativeCompass {
    angle: f64,
}

fn main() {
    // Initialize logging
    env_logger::init();
    debug!("Logging is ready");

    logger::init_logging(
        "server-log",
        &[
            "main",
            "player",
            "server_response",
            "challenge",
            "hint",
            "server_message",
        ],
    )
    .expect("Failed to initialize logging");

    // Parse command line arguments
    let matches = App::new("SQP Server")
        .version("1.0.0")
        .author("Your Name")
        .about("Server for Sauve Qui Peut game")
        .subcommand(
            SubCommand::with_name("run")
                .about("Run the server")
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .value_name("PORT")
                        .help("Port to listen on")
                        .takes_value(true)
                        .default_value("8778"),
                )
                .arg(
                    Arg::with_name("host-address")
                        .long("host-address")
                        .value_name("HOST")
                        .help("Host address to bind to")
                        .takes_value(true)
                        .default_value("127.0.0.1"),
                )
                .arg(
                    Arg::with_name("maze")
                        .long("maze")
                        .value_name("DIMENSIONS")
                        .help("Maze dimensions in format WIDTHxHEIGHT (e.g., 5,5)")
                        .takes_value(true)
                        .default_value("5,5"),
                ),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("Enable debug mode")
                .takes_value(false),
        )
        .get_matches();

    // Check for the "run" subcommand
    let run_matches = if let Some(matches) = matches.subcommand_matches("run") {
        matches
    } else {
        error!("Missing 'run' subcommand");
        std::process::exit(1);
    };

    // Extract values from arguments
    let port = run_matches
        .value_of("port")
        .unwrap()
        .parse::<u16>()
        .expect("Invalid port number");
    let host = run_matches.value_of("host-address").unwrap();
    let address = format!("{}:{}", host, port);

    // Parse maze dimensions
    let maze_dimensions = run_matches.value_of("maze").unwrap();
    let dimensions: Vec<&str> = maze_dimensions.split(',').collect();
    if dimensions.len() != 2 {
        error!("Invalid maze dimensions. Expected format: WIDTH,HEIGHT");
        std::process::exit(1);
    }

    let width = dimensions[0]
        .trim()
        .parse::<usize>()
        .expect("Invalid maze width");
    let height = dimensions[1]
        .trim()
        .parse::<usize>()
        .expect("Invalid maze height");

    // Initialize server state
    let state = Arc::new(Mutex::new(ServerState {
        teams: HashMap::new(),
        players: HashMap::new(),
        labyrinth: generate_labyrinth(width, height),
        next_player_id: 0,
    }));

    // Print the initial labyrinth
    {
        let state_lock = state.lock().unwrap();
        print_labyrinth(&state_lock);
        drop(state_lock);
    }

    info!("Server is running on {}", address);
    println!("Server is running on {}:{}", host, port);
    println!("Maze dimensions: {}x{}", width, height);

    // Start server
    match TcpListener::bind(&address) {
        Ok(listener) => {
            debug!("Listener bound successfully to {}", address);

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        debug!("New connection from {:?}", stream.peer_addr());
                        let state_clone = Arc::clone(&state);
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, state_clone) {
                                error!("Error handling client: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", address, e);
        }
    }
}

/// Generate a labyrinth using the recursive backtracking algorithm
fn generate_labyrinth(width: usize, height: usize) -> Labyrinth {
    let maze = generate_maze(width, height);

    // Convert the maze cells to our Labyrinth format
    let mut cells = Vec::with_capacity(height);

    for y in 0..height {
        let mut row = Vec::with_capacity(width);
        for x in 0..width {
            let maze_cell = &maze.cells[y][x];
            row.push(Cell {
                north_wall: maze_cell.north_wall,
                east_wall: maze_cell.east_wall,
                south_wall: maze_cell.south_wall,
                west_wall: maze_cell.west_wall,
                has_hint: maze_cell.has_hint,
                has_exit: maze_cell.has_exit,
            });
        }
        cells.push(row);
    }

    let exit_position = maze.exit_position;

    // Print info about the generated maze
    println!(
        "Created new {}x{} labyrinth with exit at ({}, {})",
        width, height, exit_position.0, exit_position.1
    );

    Labyrinth {
        width,
        height,
        cells,
        exit_position,
    }
}

// Handle client connection
fn handle_client(
    mut stream: TcpStream,
    state: Arc<Mutex<ServerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr()?;
    debug!("New connection from {}", peer_addr);

    let mut player_key: Option<String> = None;

    // Keep the connection open and handle multiple messages
    loop {
        let message_str = match receive_message(&mut stream) {
            Ok(msg) => msg,
            Err(e) => {
                match e {
                    SqpError::Network(ref ne) => {
                        if ne.to_string().contains("Connection closed by peer") {
                            debug!("Connection from {} closed by client", peer_addr);
                            break;
                        }
                    }
                    _ => {}
                }
                error!("Failed to receive message from {}: {}", peer_addr, e);
                break;
            }
        };

        debug!("Read string message: {}", message_str);

        // Parse the message using our request models
        let message: Message = match serde_json::from_str(&message_str) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse message as JSON: {}", e);
                break;
            }
        };

        // Handle different message types
        match message {
            Message::RegisterTeam(register_team) => {
                if let Err(e) = handle_register_team(&mut stream, &register_team, state.clone()) {
                    error!("Error handling team registration: {}", e);
                    break;
                }
            }
            Message::SubscribePlayer(subscribe_player) => {
                // When a player subscribes, remember their key
                let player_name = &subscribe_player.name;
                let token = &subscribe_player.registration_token;

                // Find the team with this token
                let team_name = {
                    let state = state.lock().unwrap();
                    state
                        .teams
                        .iter()
                        .find(|(_, team)| team.registration_token == *token)
                        .map(|(name, _)| name.clone())
                };

                if let Some(team_name) = team_name {
                    player_key = Some(format!("{}/{}", team_name, player_name));
                }

                if let Err(e) =
                    handle_subscribe_player(&mut stream, &subscribe_player, state.clone())
                {
                    error!("Error handling player subscription: {}", e);
                    break;
                }
            }
            Message::Action(action) => {
                if let Err(e) = handle_action(
                    &mut stream,
                    &action,
                    state.clone(),
                    peer_addr,
                    player_key.clone(),
                ) {
                    error!("Error handling player action: {}", e);
                    break;
                }
            }
        }
    }

    // Clean up player if they were registered
    if let Some(key) = player_key {
        let mut state = state.lock().unwrap();
        if state.players.remove(&key).is_some() {
            info!("Player {} disconnected and removed from game", key);
        }
    }

    debug!("Connection from {} has been closed", peer_addr);
    Ok(())
}

fn handle_register_team(
    stream: &mut TcpStream,
    message: &RegisterTeam,
    state: Arc<Mutex<ServerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Read struct message: Registration(RegisterTeam({:?}))",
        message
    );

    let team_name = &message.name;
    debug!("Subscribing for team '{}' from {:?}", team_name, stream);

    // Generate a registration token (16 hex characters)
    let registration_token = generate_token();

    // Store team information
    let mut state = state.lock().unwrap();
    state.teams.insert(
        team_name.to_string(),
        Team {
            name: team_name.to_string(),
            registration_token: registration_token.clone(),
            expected_players: 3, // Default to 3 players
            players: Vec::new(),
        },
    );

    // Create response using proper serializable structs
    let response = server_request_models::RegisterTeamResponse {
        RegisterTeamResult: server_request_models::RegisterTeamResponseResult::Ok(
            server_request_models::RegisterTeamResponseOk {
                expected_players: 3,
                registration_token: registration_token.clone(),
            },
        ),
    };

    // Send response using utility function
    debug!("Write struct message: ClientSide(Registration(RegisterTeamResult(Ok {{ expected_players: 3, registration_token: \"{}\" }})))", registration_token);

    // Send the response
    send_message(stream, &response)
        .map_err(|e| Error::Network(SendPayloadFailed(e.to_string())))?;

    Ok(())
}

fn handle_subscribe_player(
    stream: &mut TcpStream,
    message: &SubscribePlayer,
    state: Arc<Mutex<ServerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!(
        "Read struct message: Registration(SubscribePlayer({:?}))",
        message
    );

    let player_name = message.name.clone();
    let token = message.registration_token.clone();

    if let (player_name, token) = (player_name, token) {
        let mut state = state.lock().unwrap();

        // Find the team with this token
        let team_name = state
            .teams
            .iter()
            .find(|(_, team)| team.registration_token == *token)
            .map(|(name, _)| name.clone());

        if let Some(team_name) = team_name {
            debug!(
                "Subscribing for player '{}' in team '{}' from {:?}",
                player_name, team_name, stream
            );

            // Add player to team
            if let Some(team) = state.teams.get_mut(&team_name) {
                team.players.push(player_name.clone());
            }

            // Create player with initial position
            let player_id = state.next_player_id;
            state.next_player_id += 1;

            // Initialize player at different positions based on ID
            let position = match player_id % 3 {
                0 => (3, 4), // First player at (3, 4)
                1 => (4, 2), // Second player at (4, 2)
                _ => (4, 4), // Third player at (4, 4)
            };

            // Initialize player facing different directions
            let direction = match player_id % 3 {
                0 => MapDirection::West, // First player facing West
                1 => MapDirection::East, // Second player facing East
                _ => MapDirection::East, // Third player facing East
            };

            // Create and store player
            let player = Player {
                id: player_id,
                name: player_name.clone(),
                team_name: team_name.clone(),
                position,
                direction,
                moves: 0,
            };

            let player_key = format!("{}/{}", team_name, player_name);
            state.players.insert(player_key.clone(), player);

            // Store info we need for logging
            let player_position = position;
            let player_direction = direction;

            // Print the labyrinth to show player's initial position
            info!(
                "Player {} joined the game at position {:?} facing {:?}",
                player_key, player_position, player_direction
            );
            print_labyrinth(&state);

            // Send OK response
            let response = server_request_models::SubscribePlayerResponse {
                SubscribePlayerResult: server_request_models::SubscribePlayerResponseResult::Ok,
            };

            debug!("Write struct message: ClientSide(Registration(SubscribePlayerResult(Ok)))");
            send_message(stream, &response).map_err(|e| {
                error!("Failed to send subscription response: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

            // Send initial radar view
            let player = state.players.get(&player_key).unwrap();
            debug!(
                "Player {{ player_id: {} }} at {:?} towards {:?}",
                player.id, player.position, player.direction
            );

            // Generate radar view using our encode_radar_view function
            let encoded_view =
                encode_radar_view(player.position, player.direction, &state.labyrinth);

            let radar_response = server_request_models::RadarViewResponse {
                RadarView: encoded_view.clone(),
            };

            debug!(
                "Write struct message: ClientSide(Loop(RadarView(EncodedRadarView(\"{}\"))",
                encoded_view
            );
            send_message(stream, &radar_response).map_err(|e| {
                error!("Failed to send radar view: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;
        } else {
            error!("Invalid registration token: {}", token);
            // Send error response
            let response = server_request_models::SubscribePlayerResponse {
                SubscribePlayerResult: server_request_models::SubscribePlayerResponseResult::Error(
                    "Invalid registration token".to_string(),
                ),
            };

            send_message(stream, &response).map_err(|e| {
                error!("Failed to send error response: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;
        }
    } else {
        error!("Invalid SubscribePlayer message: {:?}", message);
    }

    Ok(())
}

fn handle_action(
    stream: &mut TcpStream,
    message: &Action,
    state: Arc<Mutex<ServerState>>,
    peer_addr: std::net::SocketAddr,
    player_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Read struct message: Loop(Action({:?}))", message);

    // Find the player associated with this connection
    let player_key = player_key.unwrap_or_else(|| {
        let state = state.lock().unwrap();
        state
            .players
            .iter()
            .find(|(_, player)| {
                player
                    .position
                    .0
                    .to_string()
                    .contains(&peer_addr.port().to_string())
            })
            .map(|(key, _)| key.clone())
            .unwrap_or_else(|| {
                error!("Could not find player for connection from {}", peer_addr);
                "unknown".to_string()
            })
    });

    match message {
        Action::MoveTo(direction) => {
            debug!("Action MoveTo({:?}) for '{}'", direction, player_key);

            // Values we'll collect and use after dropping the lock
            let mut hit_wall = false;
            let mut found_exit = false;
            let mut give_hint = false;
            let mut player_id = 0;
            let mut encoded_view = String::new();
            let new_position;
            let new_direction;
            let mut team_name = String::new();
            let mut player_name = String::new();
            let mut moves = 0;

            {
                let mut state_lock = state.lock().unwrap();

                // Check if player exists
                if !state_lock.players.contains_key(&player_key) {
                    error!("Player {} not found in game state", player_key);
                    return Ok(());
                }

                // First, collect all the information we need in local variables
                let current_position;
                let current_direction;

                // Scope for the immutable borrow of player to get current position/direction
                {
                    let player = state_lock.players.get(&player_key).unwrap();
                    current_position = player.position;
                    current_direction = player.direction;
                }

                // Calculate the potential new position
                let move_result = process_move(
                    current_position.0,
                    current_position.1,
                    &current_direction,
                    direction,
                );

                let mut new_x = move_result.0;
                let mut new_y = move_result.1;
                let direction = move_result.2;

                // Check for walls before allowing movement
                let mut can_move = true;
                if new_x != current_position.0 || new_y != current_position.1 {
                    // Determine which wall to check based on movement direction
                    if new_y < current_position.1 {
                        // Moving North
                        if state_lock.labyrinth.cells[current_position.1][current_position.0]
                            .north_wall
                        {
                            can_move = false;
                        }
                    } else if new_y > current_position.1 {
                        // Moving South
                        if state_lock.labyrinth.cells[current_position.1][current_position.0]
                            .south_wall
                        {
                            can_move = false;
                        }
                    } else if new_x > current_position.0 {
                        // Moving East
                        if state_lock.labyrinth.cells[current_position.1][current_position.0]
                            .east_wall
                        {
                            can_move = false;
                        }
                    } else if new_x < current_position.0 {
                        // Moving West
                        if state_lock.labyrinth.cells[current_position.1][current_position.0]
                            .west_wall
                        {
                            can_move = false;
                        }
                    }

                    // If we can't move, keep the original position
                    if !can_move {
                        hit_wall = true;
                        new_x = current_position.0;
                        new_y = current_position.1;
                        debug!("Player {} cannot move through wall", player_key);
                    }
                }

                // Get exit position for checking later
                let exit_position = state_lock.labyrinth.exit_position;

                // Now update the player with a mutable borrow
                // Scope for the mutable borrow of player to update it
                {
                    let player = state_lock.players.get_mut(&player_key).unwrap();

                    // Only update position if movement is valid
                    if can_move {
                        player.position = (new_x, new_y);
                    }

                    player.direction = direction;
                    player.moves += 1;

                    // Check if player found the exit
                    found_exit = player.position.0 == exit_position.0
                        && player.position.1 == exit_position.1;

                    // Sometimes provide a hint
                    give_hint = player.moves > 0 && player.moves % 8 == 0;
                    player_id = player.id;
                    team_name = player.team_name.clone();
                    player_name = player.name.clone();
                    moves = player.moves;

                    // Remember the player's new position and direction for generating radar view
                    new_position = player.position;
                    new_direction = player.direction;
                }

                // Generate the radar view while still holding the lock
                encoded_view =
                    encode_radar_view(new_position, new_direction, &state_lock.labyrinth);

                // Now we can safely print the labyrinth since the mutable borrow is dropped
                if can_move {
                    info!(
                        "Player {} moved to ({}, {}) facing {:?}",
                        player_key, new_x, new_y, direction
                    );
                } else {
                    info!(
                        "Player {} tried to move through a wall, stayed at position",
                        player_key
                    );
                }
                print_labyrinth(&state_lock);
            }

            if hit_wall {
                // Send wall message
                let wall_response = server_request_models::CannotPassThroughWallResponse {
                    CannotPassThroughWall: true,
                };

                send_message(stream, &wall_response).map_err(|e| {
                    error!("Failed to send wall notification: {}", e);
                    Box::new(e) as Box<dyn std::error::Error>
                })?;
            }

            if give_hint {
                // Send a hint (compass)
                let angle = rand::thread_rng().gen_range(0.0..360.0);

                let hint_response = server_request_models::HintResponse {
                    Hint: server_request_models::RelativeCompassResponse {
                        RelativeCompass: server_request_models::CompassData { angle },
                    },
                };

                debug!(
                    "Write struct message: ClientSide(Loop(Hint(RelativeCompass {{ angle: {} }})))",
                    angle
                );
                send_message(stream, &hint_response).map_err(|e| {
                    error!("Failed to send hint: {}", e);
                    Box::new(e) as Box<dyn std::error::Error>
                })?;
            }

            if found_exit {
                // Player found the exit
                info!(
                    "Team {}/{} found the exit in {} moves",
                    team_name, player_name, moves
                );

                // Send found exit message
                let exit_response = server_request_models::FoundExitResponse { FoundExit: true };

                send_message(stream, &exit_response).map_err(|e| {
                    error!("Failed to send exit notification: {}", e);
                    Box::new(e) as Box<dyn std::error::Error>
                })?;
            }

            // Always send a radar view, regardless of movement outcome
            debug!(
                "Player {{ player_id: {} }} at {:?} towards {:?} with encoded view {}",
                player_id, new_position, new_direction, encoded_view
            );

            let radar_response = server_request_models::RadarViewResponse {
                RadarView: encoded_view.clone(),
            };

            debug!(
                "Write struct message: ClientSide(Loop(RadarView(EncodedRadarView(\"{}\"))",
                encoded_view
            );
            send_message(stream, &radar_response).map_err(|e| {
                error!("Failed to send radar view: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;
        }
        Action::SolveChallenge(answer) => {
            // Handle the SolveChallenge action
            debug!("Action SolveChallenge({:?}) for '{}'", answer, player_key);
            // Implement challenge solving logic here
        }
    }

    Ok(())
}

// Process player movement
fn process_move(
    x: usize,
    y: usize,
    current_direction: &MapDirection,
    move_direction: &Direction,
) -> (usize, usize, MapDirection) {
    let (dx, dy, new_direction) = match (current_direction, move_direction) {
        // Front movement preserves direction and moves in that direction
        (MapDirection::North, Direction::Front) => (0, -1, MapDirection::North),
        (MapDirection::South, Direction::Front) => (0, 1, MapDirection::South),
        (MapDirection::East, Direction::Front) => (1, 0, MapDirection::East),
        (MapDirection::West, Direction::Front) => (-1, 0, MapDirection::West),

        // Back movement preserves direction but moves opposite
        (MapDirection::North, Direction::Back) => (0, 1, MapDirection::North),
        (MapDirection::South, Direction::Back) => (0, -1, MapDirection::South),
        (MapDirection::East, Direction::Back) => (-1, 0, MapDirection::East),
        (MapDirection::West, Direction::Back) => (1, 0, MapDirection::West),

        // Left turns 90° counter-clockwise
        (MapDirection::North, Direction::Left) => (-1, 0, MapDirection::West),
        (MapDirection::South, Direction::Left) => (1, 0, MapDirection::East),
        (MapDirection::East, Direction::Left) => (0, -1, MapDirection::North),
        (MapDirection::West, Direction::Left) => (0, 1, MapDirection::South),

        // Right turns 90° clockwise
        (MapDirection::North, Direction::Right) => (1, 0, MapDirection::East),
        (MapDirection::South, Direction::Right) => (-1, 0, MapDirection::West),
        (MapDirection::East, Direction::Right) => (0, 1, MapDirection::South),
        (MapDirection::West, Direction::Right) => (0, -1, MapDirection::North),
    };

    // Calculate potential new position
    let new_x = if dx < 0 && x > 0 {
        x - 1
    } else if dx > 0 && x < 4 {
        // Assuming 5x5 grid (0-4 indices)
        x + 1
    } else {
        x
    };

    let new_y = if dy < 0 && y > 0 {
        y - 1
    } else if dy > 0 && y < 4 {
        // Assuming 5x5 grid (0-4 indices)
        y + 1
    } else {
        y
    };

    (new_x, new_y, new_direction)
}

// Print the labyrinth to console for debugging
fn print_labyrinth(state: &ServerState) {
    let labyrinth = &state.labyrinth;
    let width = labyrinth.width;
    let height = labyrinth.height;

    println!("\n=== Labyrinth Map ===");

    // Create a grid to show player positions
    let mut display_grid: Vec<Vec<String>> = vec![vec![" ".to_string(); width]; height];

    // Mark player positions
    for (_, player) in &state.players {
        let (x, y) = player.position;
        if x < width && y < height {
            // Use direction symbols for players: ^ v > <
            let symbol = match player.direction {
                MapDirection::North => "^",
                MapDirection::South => "v",
                MapDirection::East => ">",
                MapDirection::West => "<",
            };
            display_grid[y][x] = symbol.to_string();
        }
    }

    // Mark exit position
    let (exit_x, exit_y) = labyrinth.exit_position;
    if display_grid[exit_y][exit_x] == " " {
        display_grid[exit_y][exit_x] = "X".to_string();
    }

    // Mark hints
    for y in 0..height {
        for x in 0..width {
            if labyrinth.cells[y][x].has_hint && display_grid[y][x] == " " {
                display_grid[y][x] = "H".to_string();
            }
        }
    }

    // Print top border
    print!("  ");
    for x in 0..width {
        print!("{}   ", x);
    }
    println!();

    // Print northern walls for the first row
    print!("  ");
    for x in 0..width {
        print!("+");
        if labyrinth.cells[0][x].north_wall {
            print!("---");
        } else {
            print!("   ");
        }
    }
    println!("+");

    // Print each row
    for y in 0..height {
        // Print row number
        print!("{} ", y);

        // Print cells and vertical walls
        for x in 0..width {
            // Print west wall
            if labyrinth.cells[y][x].west_wall {
                print!("|");
            } else {
                print!(" ");
            }

            // Print cell content (player or space)
            print!(" {} ", display_grid[y][x]);
        }

        // Print east wall of the last cell in the row
        if labyrinth.cells[y][width - 1].east_wall {
            println!("|");
        } else {
            println!(" ");
        }

        // Print southern walls for this row
        print!("  ");
        for x in 0..width {
            print!("+");
            if labyrinth.cells[y][x].south_wall {
                print!("---");
            } else {
                print!("   ");
            }
        }
        println!("+");
    }

    println!("Legend: ^ v > < = Players, X = Exit, H = Hint");
    println!("Players:");
    for (player_key, player) in &state.players {
        println!(
            "  {} at ({}, {}) facing {:?}, moves: {}",
            player_key, player.position.0, player.position.1, player.direction, player.moves
        );
    }
    println!();
}

// Generate a random token (16 hex characters)
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let token: String = (0..16)
        .map(|_| {
            let digit: u8 = rng.gen_range(0..16);
            format!("{:X}", digit)
        })
        .collect();
    token
}
/// Returns a 4‑bit encoded value for a cell in the labyrinth radar view.
/// Out‑of‑bounds cells are encoded as 0xF (undefined).
fn encode_cell(labyrinth: &Labyrinth, x: isize, y: isize) -> u8 {
    if x < 0 || y < 0 || (x as usize) >= labyrinth.width || (y as usize) >= labyrinth.height {
        debug!(
            "Cell at ({}, {}) is outside labyrinth bounds, encoding as solid cell (0xF)",
            x, y
        );
        return 0xF;
    }

    let cell = &labyrinth.cells[y as usize][x as usize];

    let item_bits = if cell.has_exit {
        debug!("Cell at ({}, {}) has exit", x, y);
        0b10
    } else if cell.has_hint {
        debug!("Cell at ({}, {}) has hint", x, y);
        0b01
    } else {
        debug!("Cell at ({}, {}) has no special items", x, y);
        0b00
    };

    // No entity info => lower 2 bits = 0.
    let result = (item_bits << 2) | 0b00;
    debug!(
        "Cell at ({}, {}) encoded as: {:#06b} (item bits: {:#04b}, entity bits: {:#04b})",
        x, y, result, item_bits, 0b00
    );

    result
}

/// Encode a radar view from the labyrinth for the player's 3×3 view.
/// The encoding is as follows:
/// - 12 horizontal passages (2 bits each) → 24 bits (3 bytes little‑endian)
/// - 12 vertical passages (2 bits each)   → 24 bits (3 bytes little‑endian)
/// - 9 cell values (4 bits each)            → 36 bits, then left‑shifted by 4 (padding) → 40 bits (5 bytes little‑endian)
///
/// The passages and cells are taken in natural order (top‑left first, row‑major).
pub(crate) fn encode_radar_view(
    player_position: (usize, usize),
    player_direction: MapDirection,
    labyrinth: &Labyrinth,
) -> String {
    info!(
        "Encoding radar view for player at position ({}, {}) facing {:?}",
        player_position.0, player_position.1, player_direction
    );

    // Pour l'instant, on initialise les passages à "undefined" (0xF).
    let mut horizontal_passages: u32 = 0x55_55_55; // (01 répété 12 fois)
    let mut vertical_passages: u32 = 0x55_55_55; // (01 répété 12 fois)

    debug!("Initial horizontal passages: {:#034b}", horizontal_passages);
    debug!("Initial vertical passages: {:#034b}", vertical_passages);

    let x_center = player_position.0;
    let y_center = player_position.1;

    match player_direction {
        MapDirection::North => {
            // cellule centrale.
            let center_cell = &labyrinth.cells[y_center][x_center];

            if center_cell.north_wall {
                horizontal_passages &= !(0b11 << 6);
            }
            if center_cell.east_wall {
                vertical_passages &= !(0b11 << 6);
            }
            if center_cell.south_wall {
                horizontal_passages &= !(0b11 << 8);
            }
            if center_cell.west_wall {
                vertical_passages &= !(0b11 << 4);
            }

            debug!(
                "Processing center cell at ({}, {}) with walls N:{} E:{} S:{} W:{}",
                x_center,
                y_center,
                center_cell.north_wall,
                center_cell.east_wall,
                center_cell.south_wall,
                center_cell.west_wall
            );
        }
        MapDirection::South => {
            let center_cell = &labyrinth.cells[y_center][x_center];

            if center_cell.south_wall {
                horizontal_passages &= !(0b11 << 6);
            }
            if center_cell.west_wall {
                vertical_passages &= !(0b11 << 6);
            }
            if center_cell.north_wall {
                horizontal_passages &= !(0b11 << 8);
            }
            if center_cell.east_wall {
                vertical_passages &= !(0b11 << 4);
            }
            info!("South orientation logic applied");
        }
        MapDirection::East => {
            let center_cell = &labyrinth.cells[y_center][x_center];

            if center_cell.east_wall {
                horizontal_passages &= !(0b11 << 6);
            }
            if center_cell.south_wall {
                vertical_passages &= !(0b11 << 6);
            }
            if center_cell.west_wall {
                horizontal_passages &= !(0b11 << 8);
            }
            if center_cell.north_wall {
                vertical_passages &= !(0b11 << 4);
            }
            info!("East orientation logic applied");
        }
        MapDirection::West => {
            let center_cell = &labyrinth.cells[y_center][x_center];

            if center_cell.west_wall {
                horizontal_passages &= !(0b11 << 6);
            }
            if center_cell.north_wall {
                vertical_passages &= !(0b11 << 6);
            }
            if center_cell.east_wall {
                horizontal_passages &= !(0b11 << 8);
            }
            if center_cell.south_wall {
                vertical_passages &= !(0b11 << 4);
            }
            info!("West orientation logic applied");
        }
    }

    debug!(
        "Final horizontal passages: {:#034b} (hex: {:#010x})",
        horizontal_passages, horizontal_passages
    );
    debug!(
        "Final vertical passages: {:#034b} (hex: {:#010x})",
        vertical_passages, vertical_passages
    );

    // Encodage des cellules du radar : pour chaque cellule de la grille 3×3,
    // on utilise 4 bits par cellule.
    let mut cell_values = Vec::new();
    for y_offset in -1..=1 {
        for x_offset in -1..=1 {
            let x = x_center as isize + x_offset;
            let y = y_center as isize + y_offset;
            let cell_value = encode_cell(labyrinth, x, y);
            debug!(
                "Cell at relative position ({}, {}) [absolute: ({}, {})] encoded as: {:#06b}",
                x_offset, y_offset, x, y, cell_value
            );
            cell_values.push(cell_value);
        }
    }

    // On pack les 9 valeurs de 4 bits chacune dans un entier 64 bits.
    let mut packed_cells: u64 = 0;
    for (i, &value) in cell_values.iter().enumerate() {
        packed_cells |= (value as u64) << (i * 4);
        debug!(
            "After adding cell {}: packed_cells = {:#066b}",
            i, packed_cells
        );
    }

    // Décalage à gauche de 4 bits pour le padding, comme spécifié.
    let pre_shift_packed_cells = packed_cells;
    packed_cells <<= 4;
    debug!(
        "Packed cells before shift: {:#066b}",
        pre_shift_packed_cells
    );
    debug!("Packed cells after shift: {:#066b}", packed_cells);

    let mut data = [0u8; 11];
    data[0] = (horizontal_passages & 0xFF) as u8;
    data[1] = ((horizontal_passages >> 8) & 0xFF) as u8;
    data[2] = ((horizontal_passages >> 16) & 0xFF) as u8;

    data[3] = (vertical_passages & 0xFF) as u8;
    data[4] = ((vertical_passages >> 8) & 0xFF) as u8;
    data[5] = ((vertical_passages >> 16) & 0xFF) as u8;

    data[6] = (packed_cells & 0xFF) as u8;
    data[7] = ((packed_cells >> 8) & 0xFF) as u8;
    data[8] = ((packed_cells >> 16) & 0xFF) as u8;
    data[9] = ((packed_cells >> 24) & 0xFF) as u8;
    data[10] = ((packed_cells >> 32) & 0xFF) as u8;

    debug!("Final encoded data bytes:");
    debug!(
        "Horizontal passages: [{:02X}, {:02X}, {:02X}]",
        data[0], data[1], data[2]
    );
    debug!(
        "Vertical passages: [{:02X}, {:02X}, {:02X}]",
        data[3], data[4], data[5]
    );
    debug!(
        "Cell bytes: [{:02X}, {:02X}, {:02X}, {:02X}, {:02X}]",
        data[6], data[7], data[8], data[9], data[10]
    );

    let encoded = encoder::encode(&data);
    info!("Base64 encoded result: {}", encoded);
    encoded
}

// Include tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_radar_view() {
        let cells = vec![
            vec![
                // Row 0 (top): all cells with all walls true.
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
            ],
            vec![
                // Row 1: left and right cells with walls true; center cell with no walls.
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: false,
                    east_wall: false,
                    south_wall: false,
                    west_wall: false,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
            ],
            vec![
                // Row 2 (bottom): all cells with all walls true.
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
                Cell {
                    north_wall: true,
                    east_wall: true,
                    south_wall: true,
                    west_wall: true,
                    has_hint: false,
                    has_exit: false,
                },
            ],
        ];
        let labyrinth = Labyrinth {
            width: 3,
            height: 3,
            cells,
            exit_position: (1, 1),
        };
        let player_position = (1, 1);
        let encoded = encode_radar_view(player_position, MapDirection::North, &labyrinth);
        assert_eq!(encoded, "beeqkcGO8p8p8pa");
    }
}
