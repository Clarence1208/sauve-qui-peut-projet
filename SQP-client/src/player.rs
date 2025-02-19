use crate::decoder::decode;
use crate::error::{Error, NetworkError, PlayerError};
use crate::logger::log_message;
use crate::models::{turn_left, Direction};
use crate::request_models::{Action, Answer, Message, SubscribePlayer};
use crate::server_utils::{receive_message, send_message};
use crate::SECRET_MAP;
use log::{debug, error, info, warn};
use serde_json::json;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/**
 * The Boundary enum represents the different types of boundaries in the labyrinth.
 */
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) enum Boundary {
    Undefined,
    Open,
    Wall,
    Error,
}

/**
 * The Entity enum represents the different types of entities in the labyrinth.
 */
#[derive(Debug, Eq, Hash, Clone, PartialEq)]
enum Entity {
    None,
    Ally,
    Enemy,
    Monster,
}

/**
 * The Item enum represents the different types of items in the labyrinth.
 */
#[derive(Debug, Eq, Hash, Clone, PartialEq)]
enum Item {
    None,
    Hint,
    Goal,
}

/**
 * The RadarCell struct represents a cell in the radar view.
 * It contains an item and an entity.
 * The item represents the type of item in the cell (None, Hint, Goal).
 * The entity represents the type of entity in the cell (None, Ally, Enemy, Monster).
 */
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct RadarCell {
    is_undefined: bool,
    item: Item,
    entity: Entity,
}

/**
 * The player_thread function represents the main logic for each player thread.
 * It subscribes the player to the server, then enters a loop to solve the labyrinth.
 *
 * @param player_name: String - The name of the player
 * @param registration_token: String - The registration token for the player
 * @param server_address: String - The address of the server
 */
pub(crate) fn start_player_thread(
    player_name: String,
    registration_token: String,
    server_address: String,
) -> Result<(), Error> {
    let mut player_stream = TcpStream::connect(server_address)
        .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
    println!("Connected for player: {}", player_name);

    // Subscribe the player
    let subscribe_player_message = Message::SubscribePlayer(SubscribePlayer {
        name: player_name.clone(),
        registration_token: registration_token.clone(),
    });
    send_message(&mut player_stream, &subscribe_player_message)
        .map_err(|e| PlayerError::SubscriptionFailed(e.to_string()))?;
    println!("Subscribed player: {}", player_name);

    let response = receive_message(&mut player_stream)
        .map_err(|e| PlayerError::RadarResponseFailed(e.to_string()))?;
    if !response.contains("Ok") {
        return Err(PlayerError::SubscriptionFailed(response).into());
    }
    println!("Server response for player {}: {}", player_name, response);

    // get the next response from the server that contains the radar view
    let response = receive_message(&mut player_stream)
        .map_err(|e| PlayerError::RadarResponseFailed(e.to_string()))?;
    println!(
        "Player {} received radar response: {}",
        player_name, response
    );

    search_for_exit(player_name, player_stream, response)?;

    // fixme remove, only for testing
    // choose_direction_by_hand(player_name, player_stream);

    Ok(())
}

/**
 * The search_for_exit function represents the main logic for each player to solve the labyrinth.
 * It receives the initial radar response and enters a loop to explore the labyrinth and find the exit.
 *
 * @param player_name: String - The name of the player
 * @param player_stream: TcpStream - The TCP stream for the player
 * @param initial_radar_response: String - The initial radar response from the server
 */
fn search_for_exit(
    player_name: String,
    mut player_stream: TcpStream,
    initial_radar_response: String,
) -> Result<(), Error> {
    // Parse the radar to get the initial state of the labyrinth
    let (mut _cells, mut horizontal_passages, mut vertical_passages) =
        parse_radar_response(&initial_radar_response);
    // Initial player direction
    let mut current_direction = Direction::Right; // always try to go right first

    // main loop for player movement
    loop {
        // check if the player can go right else try front then left then back
        while !is_direction_open(&current_direction, &horizontal_passages, &vertical_passages) {
            current_direction = turn_left(&current_direction);
        }
        // Send the current movement action
        let action_message = Message::Action(Action::MoveTo(current_direction.clone()));

        send_message(&mut player_stream, &action_message)
            .map_err(|e| PlayerError::ActionFailed(e.to_string()))?;
        println!(
            "Player {} sent action: {:?}",
            player_name, current_direction
        );

        // Receive the server's response to the action
        let mut action_response = receive_message(&mut player_stream)
            .map_err(|e| PlayerError::RadarResponseFailed(e.to_string()))?;
        println!(
            "Player {} received response: {}",
            player_name, action_response
        );

        if action_response.contains("Hint") {
            println!("Player {} found a hint!", player_name);
            handle_hint(&player_name, &action_response)?;

            // get next message from server to get the radar view
            action_response = receive_message(&mut player_stream)
                .map_err(|e| PlayerError::RadarResponseFailed(e.to_string()))?;
            println!(
                "Player {} received response: {}",
                player_name, action_response
            );
        }

        if action_response.contains("Challenge") {
            println!("Player {} found a challenge!", player_name);
            // cannot move until challenge is solved
            resolve_challenge(&player_name, &mut player_stream, &action_response)?;

            // get next message from server to get the radar view
            action_response = receive_message(&mut player_stream)
                .map_err(|e| PlayerError::RadarResponseFailed(e.to_string()))?;
            if action_response.contains("RadarView") {
                // Log the challenge solution in projectRoot/log/challenge.log
                log_message(
                    "challenge",
                    &format!("Player {} successfully solved the challenge\n", player_name),
                )?;
            }
        }

        player_stream.flush().map_err(|e| PlayerError::ActionFailed(e.to_string()))?;

        // Check for exit condition
        if action_response.contains("FoundExit") {
            println!("Player {} found the exit!", player_name);
            // terminate the player thread
            return Ok(());
        }

        // parse and update cells, horizontal and vertical passages
        (_cells, horizontal_passages, vertical_passages) = parse_radar_response(&action_response);
        current_direction = Direction::Right; // Reset the direction to right

        // timeout 1/100 of a second
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Check if movement was blocked
        if action_response.contains("CannotPassThroughWall") {
            // throw error
            eprintln!(
                "Player {} hit a wall, turning to {:?}",
                player_name, current_direction
            );
        }
    }
}

fn handle_hint(player_name: &String, hint: &String) -> Result<(), Error> {
    // Log the hint in projectRoot/log/hint.log
    debug!("Received a hint: {}", hint);

    // Parse secret from hint if present
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(hint) {
        if let Some(secret_val) = json_val["Hint"]["Secret"].as_u64() {
            if let Some(map) = SECRET_MAP.get() {
                let mut map = map.lock().map_err(|e| PlayerError::HintHandlingFailed(e.to_string()))?;
                map.insert(player_name.clone(), secret_val);
                info!("Stored secret for player {}: {}", player_name, secret_val);
            }
        }
    }

    let log_dir = "log";
    if let Err(e) = std::fs::create_dir_all(log_dir) {
        error!("Failed to create log directory: {}", e);
        return Ok(());
    }

    let hint_log_file = format!("{}/hint.log", log_dir);
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&hint_log_file)
        .and_then(|mut file| {
            file.write_all(format!("Player: {} found:{}\n", player_name, hint).as_bytes())
        })
    {
        error!("Failed to write hint to {}: {}", hint_log_file, e);
    } else {
        info!("Hint logged to {}", hint_log_file);
    }

    Ok(())
}

fn resolve_challenge(player_name: &String, player_stream: &mut TcpStream, challenge: &String) -> Result<(), Error> {
    // Try to read "Modulo" first, if not present, try "SecretSumModulo"
    let json_val = serde_json::from_str::<serde_json::Value>(challenge)
        .map_err(|e| PlayerError::ChallengeResolutionFailed(e.to_string()))?;

    let mod_val = json_val["Challenge"]["Modulo"]
        .as_u64()
        .or(json_val["Challenge"]["SecretSumModulo"].as_u64())
        .ok_or_else(|| PlayerError::ChallengeResolutionFailed("Missing modulo value in challenge".to_string()))?;

    if let Some(map) = SECRET_MAP.get() {
        // Now we have the modulo value from the challenge
        let map = map.lock().map_err(|e| PlayerError::ChallengeResolutionFailed(e.to_string()))?;
        let secret_hints: Vec<&u64> = map
            .iter()
            // .filter(|(name, _)| *name != player_name)
            .map(|(_, secret)| secret)
            // log for debugging
            .inspect(|secret| println!("Secret: {}", secret))
            .collect();

        // Calculate the sum of the secret hints
        let sum_of_secret_hint: u128 = secret_hints.iter().map(|&hint| *hint as u128).sum();

        let modulo_result = (sum_of_secret_hint % mod_val as u128) as u64;

        println!(
            "Player {} resolving challenge with sum {} modulo {} = {}",
            player_name, sum_of_secret_hint, mod_val, modulo_result
        );

        // Construct solution message
        let solution_message = Message::Action(Action::SolveChallenge(Answer {
            answer: modulo_result.to_string(),
        }));

        // Send the solution message
        send_message(player_stream, &solution_message)
            .map_err(|e| PlayerError::ActionFailed(e.to_string()))?;
        info!(
            "Sent challenge solution for player {}: {}",
            player_name, modulo_result
        );

        // Log the challenge solution
        log_message(
            "challenge",
            &format!("Player {} found: {}", player_name, challenge),
        )?;
    }

    Ok(())
}

// fixme remove, only for testing
// waiting for user input 1,2,3 or 4
fn choose_direction_by_hand(player_name: String, mut player_stream: TcpStream) {
    let mut current_direction = Direction::Right;
    loop {
        // 1 = front, 2 = right, 3 = back, 4 = left
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let input = input.trim();
        match input {
            "1" => current_direction = Direction::Front,
            "2" => current_direction = Direction::Right,
            "3" => current_direction = Direction::Back,
            "4" => current_direction = Direction::Left,
            _ => println!("Invalid input"),
        }

        let action_message = Message::Action(Action::MoveTo(current_direction.clone()));

        send_message(&mut player_stream, &action_message).expect("Failed to send action");
        println!(
            "Player {} sent action: {:?}",
            player_name, current_direction
        );

        // Receive the server's response to the action
        let action_response =
            receive_message(&mut player_stream).expect("Failed to receive action response");
        println!(
            "Player {} received response: {}",
            player_name, action_response
        );

        player_stream.flush().expect("Failed to flush stream");

        parse_radar_response(&action_response);
    }
}

/**
 * The is_direction_open function checks if the player can move in the given direction.
 * It takes the next direction, the horizontal passages, and the vertical passages as input.
 * It returns true if the direction is open, false otherwise.
 */
fn is_direction_open(
    next_direction: &Direction,
    h_passages: &[Boundary],
    v_passages: &[Boundary],
) -> bool {
    // We know the player is in the center cell of the radar view.
    // We are following the right-hand rule, so we want to check the passage to the right of the player.

    // Map the next direction to the passage index
    let passage_index = match next_direction {
        Direction::Front => 4,
        Direction::Right => 6,
        Direction::Back => 7,
        Direction::Left => 5,
    };

    // log for debugging
    println!(
        "Checking passage for direction {:?} (index {})",
        next_direction, passage_index
    );

    // if the direction is front or back log the horizontal passages
    if next_direction == &Direction::Front || next_direction == &Direction::Back {
        println!("Horizontal Passages:");
        for (i, passage) in h_passages.iter().clone().enumerate() {
            println!("  Passage {}: {:?}", i, passage);
        }
    }

    // if the direction is left or right log the vertical passages
    if next_direction == &Direction::Left || next_direction == &Direction::Right {
        println!("Vertical Passages:");
        for (i, passage) in v_passages.iter().clone().enumerate() {
            println!("  Passage {}: {:?}", i, passage);
        }
    }

    // passage checked (should always be open)
    let passage = match next_direction {
        Direction::Front | Direction::Back => &h_passages[passage_index].clone(),
        Direction::Left | Direction::Right => &v_passages[passage_index].clone(),
    };

    println!("Passage checked: {:?}", passage);

    matches!(passage, Boundary::Open)
}

/**
 * The parse_radar_response function parses the radar response from the server.
 * It extracts the radar data from the response, decodes the data, and parses the cells, horizontal passages, and vertical passages.
 * It returns a tuple containing the cells, horizontal passages, and vertical passages.
 */
pub(crate) fn parse_radar_response(
    response: &str,
) -> (Vec<RadarCell>, Vec<Boundary>, Vec<Boundary>) {
    if response.contains("CannotPassThroughWall")
        || response.contains("FoundExit")
        || response.contains("Hint")
    {
        return (vec![], vec![], vec![]);
    }

    // Extract radar data from the response
    // Response format: {"RadarView":"aeQrajHOapap//a"}
    let radar_data = response
        .split("\":\"")
        .nth(1)
        .unwrap()
        .split("\"")
        .next()
        .unwrap();

    if radar_data.is_empty() {
        println!("No radar data found in the response.");
        panic!("No radar data found in the response.");
    }

    // Decode the radar data
    let decoded_radar_data = decode(radar_data).expect("Failed to decode radar data");

    // Print the decoded radar data
    println!("Decoded radar data: {:?}", decoded_radar_data);

    // Check that the length of the decoded data is 11 bytes
    // (3 bytes for horizontal passages, 3 bytes for vertical passages, 5 bytes for cells)
    if decoded_radar_data.len() != 11 {
        println!("Invalid radar data length: {}", decoded_radar_data.len());
        panic!("Invalid radar data length: {}", decoded_radar_data.len());
    }

    // Parse the horizontal passages (12 passages, 2 bits each)
    let horizontal_passages = parse_passages(&decoded_radar_data[0..3], 12, "Horizontal");

    // Parse les passages verticaux (12 passages, 2 bits chacun)
    let vertical_passages = parse_passages(&decoded_radar_data[3..6], 12, "Vertical");

    // Parse les cellules (9 cellules, 4 bits chacune)
    let cells = parse_cells(&decoded_radar_data[6..11]);

    println!("Horizontal Passages:");
    for (i, passage) in horizontal_passages.iter().enumerate() {
        println!("  Passage {}: {:?}", i, passage);
    }

    println!("Vertical Passages:");
    for (i, passage) in vertical_passages.iter().enumerate() {
        println!("  Passage {}: {:?}", i, passage);
    }

    println!("Cells:");
    for (i, cell) in cells.iter().enumerate() {
        println!("  Cell {}: {:?}", i, cell);
    }

    let two_d_cells: Vec<Vec<RadarCell>> = cells.chunks(3).map(|chunk| chunk.to_vec()).collect();

    // print radar map
    println!(
        "{}",
        get_radar_map_as_string(&two_d_cells, &horizontal_passages, &vertical_passages)
    );

    (cells, horizontal_passages, vertical_passages)
}

/**
 * The parse_passages function extracts the passages from the radar data.<br>
 * It rearranges the bytes to extract the passages, then extracts the passages 2 bits at a time.<br>
 * The passages are represented by the Boundary enum. The function returns a vector of Boundary values.<br>
 * If the radar data is empty or the number of passages is 0, the function returns an empty vector.<br>
 * If the passage bits are invalid, the function returns a vector with BoundaryError values.<br>
 * The function logs the original bytes, the rearranged bytes, and the extracted passages for debugging.<br>
 */
fn parse_passages(bytes: &[u8], num_passages: usize, passage_type: &str) -> Vec<Boundary> {
    if bytes.is_empty() || num_passages == 0 {
        return vec![];
    }

    let mut passages = Vec::with_capacity(num_passages);

    // Log bytes before rearrangement
    println!("{} original bytes (hex): {:02X?}", passage_type, bytes);
    println!(
        "{} original bytes (bin): {:?}",
        passage_type,
        bytes
            .iter()
            .map(|b| format!("{:08b}", b))
            .collect::<Vec<String>>()
    );

    // Rearrange bytes to extract passages
    let bits = ((bytes[2] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[0] as u32);

    // Log bytes after rearrangement
    let bytes_be = [(bits >> 16) as u8, (bits >> 8) as u8, bits as u8];
    println!(
        "{} bytes after rearrangement (big endian order): {:02X?}",
        passage_type, bytes_be
    );
    println!(
        "{} bytes after rearrangement (big endian order, bin): {:?}",
        passage_type,
        bytes_be
            .iter()
            .map(|b| format!("{:08b}", b))
            .collect::<Vec<String>>()
    );

    // Extract passages from bits, 2 bits at a time
    for i in 0..num_passages {
        let shift = (num_passages - 1 - i) * 2;
        let passage_bits = ((bits >> shift) & 0b11) as u8;
        let passage = match passage_bits {
            0 => Boundary::Undefined,
            1 => Boundary::Open,
            2 => Boundary::Wall,
            _ => Boundary::Error, // Error value for 0b11
        };
        passages.push(passage);
    }

    // log for debugging
    log::debug!("{} extracted passages: {:?}", passage_type, passages);

    passages
}

fn parse_cells(data: &[u8]) -> Vec<RadarCell> {
    let mut cells = Vec::new();
    let mut bits = 0u64;
    for &byte in data {
        bits = (bits << 8) | byte as u64;
    }

    // The 4 padding bits are the 4 least significant bits
    bits >>= 4;

    for i in (0..9).rev() {
        let value = (bits >> (i * 4)) & 0b1111;
        if value == 0b1111 {
            // Donnée invalide ou non définie
            cells.push(RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            });
            continue;
        }

        let item_bits = (value >> 2) & 0b11;
        let entity_bits = value & 0b11;

        let item = match item_bits {
            0b00 => Item::None,
            0b01 => Item::Hint,
            0b10 => Item::Goal,
            _ => Item::None,
        };

        let entity = match entity_bits {
            0b00 => Entity::None,
            0b01 => Entity::Ally,
            0b10 => Entity::Enemy,
            0b11 => Entity::Monster,
            _ => Entity::None,
        };

        cells.push(RadarCell {
            is_undefined: false,
            item,
            entity,
        });
    }

    cells
}

/// The get_radar_map_as_string function generates a string representation of the radar map.<br>
/// It takes the radar cells, horizontal passages, and vertical passages as input.<br>
/// It constructs the map line by line, using symbols to represent the different elements:
/// - '#' for undefined cells and passages
/// - ' ' for defined cells and open passages
/// - '-' for walls in horizontal passages
/// - '|' for walls in vertical passages
/// - '•' for joints between passages
/// It returns the radar map as a string.
///
/// @param cells: &Vec<RadarCell> - The radar cells (9 cells)<br>
/// @param h_passages: &[Boundary] - The horizontal passages (12 passages)<br>
/// @param v_passages: &[Boundary] - The vertical passages (12 passages)<br>
fn get_radar_map_as_string(
    cells: &Vec<Vec<RadarCell>>,
    h_passages: &[Boundary],
    v_passages: &[Boundary],
) -> String {
    // Symbol mappings
    let symbols_cells = std::collections::HashMap::from([(true, '#'), (false, ' ')]);

    let joint = '•';

    let symbols_passages_horizontal = std::collections::HashMap::from([
        (Boundary::Undefined, '#'),
        (Boundary::Open, ' '),
        (Boundary::Wall, '-'),
    ]);

    let symboles_passages_vertical = std::collections::HashMap::from([
        (Boundary::Undefined, '#'),
        (Boundary::Open, ' '),
        (Boundary::Wall, '|'),
    ]);

    let mut carte: Vec<String> = Vec::new();

    // Convert v_passages to a 2D array (3x4)
    let mut passages_verticaux = vec![vec![Boundary::Undefined; 4]; 3];
    for i in 0..3 {
        for j in 0..4 {
            passages_verticaux[i][j] = v_passages[i * 4 + j].clone();
        }
    }

    // Convert h_passages to a 2D array (4x3)
    let mut passages_horizontaux = vec![vec![Boundary::Undefined; 3]; 4];
    for i in 0..4 {
        for j in 0..3 {
            passages_horizontaux[i][j] = h_passages[i * 3 + j].clone();
        }
    }

    // Construct the radar map line by line
    // seven iterations for each line
    for i in 0..7 {
        // Line of cells
        let mut ligne = String::new();

        if i % 2 == 0 {
            // seven iterations for each line
            for j in 0..7 {
                // if j is not pair check if joint char is needed '•'
                if j % 2 != 0 {
                    ligne.push(
                        *symbols_passages_horizontal
                            .get(&passages_horizontaux[i / 2][j / 2])
                            .unwrap(),
                    );
                } else {
                    // to check if joint is needed ->
                    // if first half of the line, check the passage after, if open '•' else '#'
                    // if second half of the line, check the passage before, if open '•' else '#'
                    if j < 3 {
                        ligne.push(
                            if passages_horizontaux[i / 2][j / 2] != Boundary::Undefined
                                || (j != 0
                                    && passages_horizontaux[i / 2][(j - 1) / 2]
                                        != Boundary::Undefined)
                            {
                                joint
                            } else {
                                '#'
                            },
                        );
                    } else {
                        ligne.push(
                            if passages_horizontaux[i / 2][(j - 1) / 2] != Boundary::Undefined
                                || (j != 6
                                    && passages_horizontaux[i / 2][j / 2] != Boundary::Undefined)
                            {
                                joint
                            } else {
                                '#'
                            },
                        );
                    }
                }
            }
        } else {
            // Line of vertical passages
            // seven iterations for each line
            for j in 0..7 {
                // if j is not pair place the value of the vertical passage / 2
                // else place the value of the cell / 2
                if j % 2 == 0 {
                    ligne.push(
                        *symboles_passages_vertical
                            .get(&passages_verticaux[(i - 1) / 2][j / 2])
                            .unwrap(),
                    );
                } else {
                    ligne.push(
                        *symbols_cells
                            .get(&cells[i / 2][j / 2].is_undefined)
                            .unwrap(),
                    );
                }
            }
        }

        carte.push(ligne);
    }

    // return map joined by return char '\n' and a return char at the end
    carte.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_passages_empty() {
        let data = [];
        let passages = parse_passages(&data, 0, "horizontal");
        assert!(passages.is_empty());
    }

    #[test]
    fn test_parse_passages_all_undefined() {
        let data = [0x00, 0x00, 0x00];
        let passages = parse_passages(&data, 12, "horizontal");
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Undefined);
        }
    }

    #[test]
    fn test_parse_passages_all_open() {
        let data = [0x55, 0x55, 0x55]; // 0b01010101 01010101 01010101
        let passages = parse_passages(&data, 12, "horizontal");
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Open);
        }
    }

    #[test]
    fn test_parse_passages_all_wall() {
        let data = [0xAA, 0xAA, 0xAA]; // 0b10101010 10101010 10101010
        let passages = parse_passages(&data, 12, "horizontal");
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Wall);
        }
    }

    #[test]
    fn test_parse_passages_mixed() {
        let data = [0b01001000, 0b00010010, 0b10010000];
        let passages = parse_passages(&data, 12, "vertical");
        let data2 = [0b00100000, 0b01000110, 0b00010010];
        parse_passages(&data2, 12, "vertical");
        // 10010000 00010010 01001000
        // ["10010000", "00010010", "01001000"]
        let expected = vec![
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Undefined,
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_passage_real_case() {
        //00100000 01000110 00010010
        let data = [0b00100000, 0b01000110, 0b00010010];
        // inverted should be 00010010 01000110 00100000
        let passages = parse_passages(&data, 12, "horizontal");
        let expected = vec![
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_passages_real_case() {
        // 10000000 10011000 00101000
        let data = [0b10000000, 0b10011000, 0b00101000];
        let passages = parse_passages(&data, 12, "horizontal");
        // inverted should be 00101000 10011000 10000000
        let expected = vec![
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_passages_specific_case() {
        // Exemple avec une séquence spécifique
        let data = [0b00011011, 0b01100110, 0b11001100];
        // inverse bit (little endian to big endian)
        // 11001100 01100110 00011011
        let passages = parse_passages(&data, 12, "horizontal");
        let expected = vec![
            Boundary::Error,
            Boundary::Undefined,
            Boundary::Error,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Error,
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_passages_invalid_values() {
        let data = [0b11111111, 0b11111111, 0b11111111];
        let passages = parse_passages(&data, 12, "horizontal");
        assert_eq!(passages.len(), 12);
        let expected = vec![
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
            Boundary::Error,
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_message_without_error() {
        let data = [0b00011010, 0b01100110, 0b10000100];
        let passages = parse_passages(&data, 12, "horizontal");
        // inverted should be 10000100 01100110 00011010
        let expected = vec![
            // first byte
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            // second byte
            Boundary::Open,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            // third byte
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Wall,
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn is_direction_open_test() {
        let h_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        let v_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        assert_eq!(
            is_direction_open(&Direction::Front, &h_passages, &v_passages),
            true
        );
        assert_eq!(
            is_direction_open(&Direction::Right, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Back, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Left, &h_passages, &v_passages),
            false
        );

        let h_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        let v_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        assert_eq!(
            is_direction_open(&Direction::Back, &h_passages, &v_passages),
            true
        );
        assert_eq!(
            is_direction_open(&Direction::Front, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Right, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Left, &h_passages, &v_passages),
            false
        );

        let h_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        let v_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        assert_eq!(
            is_direction_open(&Direction::Right, &h_passages, &v_passages),
            true
        );
        assert_eq!(
            is_direction_open(&Direction::Front, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Back, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Left, &h_passages, &v_passages),
            false
        );

        let h_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        let v_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
        ];

        assert_eq!(
            is_direction_open(&Direction::Left, &h_passages, &v_passages),
            true
        );
        assert_eq!(
            is_direction_open(&Direction::Front, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Right, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Back, &h_passages, &v_passages),
            false
        );

        let h_passages = vec![
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        let v_passages = vec![
            Boundary::Open,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        assert_eq!(
            is_direction_open(&Direction::Front, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Right, &h_passages, &v_passages),
            false
        );
        assert_eq!(
            is_direction_open(&Direction::Back, &h_passages, &v_passages),
            true
        );
        assert_eq!(
            is_direction_open(&Direction::Left, &h_passages, &v_passages),
            true
        );
    }

    #[test]
    fn test_print_map() {
        // Passages horizontaux (en regroupant par 2 bits consécutifs):
        //     Undefined, Open, Undefined (ligne 1),
        // Wall, Open, Undefined (ligne 2),
        // Open, Wall, Undefined (ligne 3),
        // Wall, Undefined, Undefined (ligne 4).
        //     Passages verticaux (en regroupant par 2 bits consécutifs):
        //     Undefined, Wall, Wall, Undefined (ligne 1)
        // Wall, Open, Wall, Undefined (ligne 2),
        // Wall, Undefined, Undefined, Undefined (ligne 3).
        //     Les cellules (une cellule par caractère hexa):
        //     Undefined, Rien, Undefined (ligne 1),
        // Rien, Rien (votre position), Undefined (ligne 2),
        // Rien, Undefined, Undefined (ligne 3).

        let cells = vec![
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
        ];

        let horizontal_passages = vec![
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        let vertical_passages = vec![
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Open,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Undefined,
        ];

        let expected = "\
        ##• •##\n\
        ##| |##\n\
        •-• •##\n\
        |   |##\n\
        • •-•##\n\
        | #####\n\
        •-•####\n";

        let two_d_cells: Vec<Vec<RadarCell>> = cells
            .chunks(3)
            .map(|chunk| chunk.to_vec()) // Convert the slice to an owned Vec<RadarCell>
            .collect(); // Collect into a Vec<Vec<RadarCell>>

        let result =
            get_radar_map_as_string(&two_d_cells, &horizontal_passages, &vertical_passages);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_print_map_straight_line() {
        let cells = vec![
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: false,
                item: Item::None,
                entity: Entity::None,
            },
            RadarCell {
                is_undefined: true,
                item: Item::None,
                entity: Entity::None,
            },
        ];

        let horizontal_passages = vec![
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Open,
            Boundary::Undefined,
        ];

        let vertical_passages = vec![
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Undefined,
            Boundary::Undefined,
            Boundary::Wall,
            Boundary::Wall,
            Boundary::Undefined,
        ];

        let expected = "\
        ##• •##\n\
        ##| |##\n\
        ##• •##\n\
        ##| |##\n\
        ##• •##\n\
        ##| |##\n\
        ##• •##\n";

        let two_d_cells: Vec<Vec<RadarCell>> = cells
            .chunks(3)
            .map(|chunk| chunk.to_vec()) // Convert the slice to an owned Vec<RadarCell>
            .collect(); // Collect into a Vec<Vec<RadarCell>>

        let result =
            get_radar_map_as_string(&two_d_cells, &horizontal_passages, &vertical_passages);

        assert_eq!(result, expected);
    }
}
