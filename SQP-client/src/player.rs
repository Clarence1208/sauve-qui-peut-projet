use crate::decoder::decode;
use crate::server_utils::{receive_message, send_message};
use crate::{Action, Direction, Message, SubscribePlayer};
use std::cmp::PartialEq;
use std::net::TcpStream;

#[derive(Debug)]
enum Boundary {
    Undefined,
    Open,
    Wall,
    Error,
}

impl PartialEq for Boundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Boundary::Undefined, Boundary::Undefined) => true,
            (Boundary::Open, Boundary::Open) => true,
            (Boundary::Wall, Boundary::Wall) => true,
            (Boundary::Error, Boundary::Error) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum Entity {
    None,
    Ally,
    Enemy,
    Monster,
}

#[derive(Debug)]
enum Item {
    None,
    Hint,
    Goal,
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Item::None, Item::None) => true,
            (Item::Hint, Item::Hint) => true,
            (Item::Goal, Item::Goal) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct RadarCell {
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
pub(crate) fn player_thread(player_name: String, registration_token: String, server_address: String) {
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

    // Labyrinth-solving loop
    let mut current_direction = Direction::Front; // Start by trying to move forward
    let mut blocked_count = 0; // Count the number of consecutive blocked movements

    // loop {
        // Send the current movement action
        let action_message = Message::Action(Action {
            MoveTo: current_direction.clone(),
        });
        send_message(&mut player_stream, &action_message).expect("Failed to send action");
        println!("Player {} sent action: {:?}", player_name, current_direction);

        // Receive the server's response to the action
        let action_response = receive_message(&mut player_stream).expect("Failed to receive action response");
        println!("Player {} received response: {}", player_name, action_response);

        parse_radar_response(&action_response);

        // timeout 1/100 of a second
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Check for exit condition
        if action_response.contains("FoundExit") {
            println!("Player {} found the exit!", player_name);
            // break; // Exit the loop
        }

        // Check if movement was blocked
        if action_response.contains("CannotPassThroughWall") {
            if blocked_count > 2 {
                // reset blocked count
                blocked_count = 0;
                // turn around
                current_direction = match current_direction {
                    Direction::Front => Direction::Back,
                    Direction::Right => Direction::Left,
                    Direction::Back => Direction::Front,
                    Direction::Left => Direction::Right,
                };
            } else {

                // If movement is blocked, turn right
                current_direction = match current_direction {
                    Direction::Front => Direction::Right,
                    Direction::Right => Direction::Back,
                    Direction::Back => Direction::Left,
                    Direction::Left => Direction::Front,
                };
            }
            blocked_count += 1;
            println!("Player {} hit a wall, turning to {:?}", player_name, current_direction);
        }

        // fixme for testing waiting for user input 1,2,3 or 4
    // 1 = front, 2 = right, 3 = back, 4 = left
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");
    let input = input.trim();
    match input {
        "1" => current_direction = Direction::Front,
        "2" => current_direction = Direction::Right,
        "3" => current_direction = Direction::Back,
        "4" => current_direction = Direction::Left,
        _ => println!("Invalid input"),
    }

    let action_message = Message::Action(Action {
        MoveTo: current_direction.clone(),
    });
    send_message(&mut player_stream, &action_message).expect("Failed to send action");
    println!("Player {} sent action: {:?}", player_name, current_direction);

    // Receive the server's response to the action
    let action_response = receive_message(&mut player_stream).expect("Failed to receive action response");
    println!("Player {} received response: {}", player_name, action_response);

    parse_radar_response(&action_response);

    // }
}

pub(crate) fn parse_radar_response(response: &str) {
    if response.contains("CannotPassThroughWall")
        || response.contains("FoundExit")
        || response.contains("Hint") {
        return;
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

    // Print the encoded radar data
    println!("Radar data: {}", radar_data);

    // Decode the radar data
    let decoded_radar_data = decode(&radar_data).expect("Failed to decode radar data");

    // Print the decoded radar data
    println!("Decoded radar data: {:?}", decoded_radar_data);

    // Check that the length of the decoded data is 11 bytes
    // (3 bytes for horizontal passages, 3 bytes for vertical passages, 5 bytes for cells)
    if decoded_radar_data.len() != 11 {
        println!("Invalid radar data length: {}", decoded_radar_data.len());
        return;
    }

    // Parse the horizontal passages (12 passages, 2 bits each)
    let horizontal_passages = parse_passages(&decoded_radar_data[0..3], 12);

    // Parse les passages verticaux (12 passages, 2 bits chacun)
    let vertical_passages = parse_passages(&decoded_radar_data[3..6], 12);

    // Parse les cellules (9 cellules, 4 bits chacune)
    let cells = parse_cells(&decoded_radar_data[6..11]);

    // Afficher les passages horizontaux
    println!("Horizontal Passages:");
    for (i, passage) in horizontal_passages.iter().enumerate() {
        println!("  Passage {}: {:?}", i, passage);
    }

    // Afficher les passages verticaux
    println!("Vertical Passages:");
    for (i, passage) in vertical_passages.iter().enumerate() {
        println!("  Passage {}: {:?}", i, passage);
    }

    // Afficher les cellules
    println!("Cells:");
    for (i, cell) in cells.iter().enumerate() {
        println!("  Cell {}: {:?}", i, cell);
    }

    // Afficher une représentation de la carte
    print_radar_map(&cells, &horizontal_passages, &vertical_passages);
}

fn parse_passages(data: &[u8], count: usize) -> Vec<Boundary> {
    let mut passages = Vec::new();
    let mut bits = 0u32;
    for &byte in data {
        bits = (bits << 8) | byte as u32;
    }

    for i in (0..count).rev() {
        let value = (bits >> (i * 2)) & 0b11;
        let passage = match value {
            0 => Boundary::Undefined,
            1 => Boundary::Open,
            2 => Boundary::Wall,
            // invalid value (should not happen throw an error)
            _ => Boundary::Error,
        };
        if passage == Boundary::Error {
            println!("Invalid passage value: {}", value);
        }
        passages.push(passage);
    }

    passages
}

fn parse_cells(data: &[u8]) -> Vec<RadarCell> {
    let mut cells = Vec::new();
    let mut bits = 0u64;
    for &byte in data {
        bits = (bits << 8) | byte as u64;
    }

    // Les 4 bits de padding sont les 4 bits de poids faible
    bits = bits >> 4;

    for i in (0..9).rev() {
        let value = (bits >> (i * 4)) & 0b1111;
        if value == 0b1111 {
            // Donnée invalide ou non définie
            cells.push(RadarCell {
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

        cells.push(RadarCell { item, entity });
    }

    cells
}

fn print_radar_map(cells: &Vec<RadarCell>, h_passages: &Vec<Boundary>, v_passages: &Vec<Boundary>) {
    // Symboles pour les cellules
    let cell_symbols = |cell: &RadarCell| {
        if cell.item == Item::Goal {
            'G'
        } else if cell.item == Item::Hint {
            'H'
        } else {
            match cell.entity {
                Entity::Ally => 'A',
                Entity::Enemy => 'E',
                Entity::Monster => 'M',
                Entity::None => ' ',
            }
        }
    };

    // Symboles pour les passages
    let passage_symbol = |passage: &Boundary, horizontal: bool| match passage {
        Boundary::Undefined => '?',
        Boundary::Open => ' ',
        Boundary::Wall => if horizontal { '-' } else { '|' },
        Boundary::Error => '!',
    };

    // Créer une grille 7x7 pour représenter le radar
    let mut grid = vec![vec![' '; 7]; 7];

    // Positions des cellules
    let cell_positions = [
        (1, 1), (1, 3), (1, 5),
        (3, 1), (3, 3), (3, 5),
        (5, 1), (5, 3), (5, 5),
    ];

    // Remplir les cellules
    for (i, &(row, col)) in cell_positions.iter().enumerate() {
        let cell = &cells[i];
        grid[row][col] = cell_symbols(cell);
    }

    // Positions des passages horizontaux
    let h_pass_positions = [
        (0, 1), (0, 3), (0, 5),
        (2, 1), (2, 3), (2, 5),
        (4, 1), (4, 3), (4, 5),
        (6, 1), (6, 3), (6, 5),
    ];

    // Remplir les passages horizontaux
    for (i, &(row, col)) in h_pass_positions.iter().enumerate() {
        let passage = &h_passages[i];
        grid[row][col] = passage_symbol(passage, true);
    }

    // Positions des passages verticaux
    let v_pass_positions = [
        (1, 0), (1, 2), (1, 4), (1, 6),
        (3, 0), (3, 2), (3, 4), (3, 6),
        (5, 0), (5, 2), (5, 4), (5, 6),
    ];

    // Remplir les passages verticaux
    for (i, &(row, col)) in v_pass_positions.iter().enumerate() {
        let passage = &v_passages[i];
        grid[row][col] = passage_symbol(passage, false);
    }

    // Afficher la carte
    println!("Radar Map:");
    for row in grid {
        let line: String = row.into_iter().collect();
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_passages_empty() {
        let data = [];
        let passages = parse_passages(&data, 0);
        assert!(passages.is_empty());
    }

    #[test]
    fn test_parse_passages_all_undefined() {
        let data = [0x00, 0x00, 0x00];
        let passages = parse_passages(&data, 12);
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Undefined);
        }
    }

    #[test]
    fn test_parse_passages_all_open() {
        let data = [0x55, 0x55, 0x55]; // 0b01010101 01010101 01010101
        let passages = parse_passages(&data, 12);
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Open);
        }
    }

    #[test]
    fn test_parse_passages_all_wall() {
        let data = [0xAA, 0xAA, 0xAA]; // 0b10101010 10101010 10101010
        let passages = parse_passages(&data, 12);
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Wall);
        }
    }

    #[test]
    fn test_parse_passages_mixed() {
        let data = [0b11001100, 0b00110011, 0b11110000];
        let passages = parse_passages(&data, 12);
        let expected = vec![
            Boundary::Error, // bits 23-22: 11 (value 3)
            Boundary::Undefined, // bits 21-20: 00 (value 0)
            Boundary::Error, // bits 19-18: 11 (value 3)
            Boundary::Undefined, // bits 17-16: 00 (value 0)
            Boundary::Undefined, // bits 21-20: 00 (value 0)
            Boundary::Error, // bits 19-18: 11 (value 3)
            Boundary::Undefined, // bits 17-16: 00 (value 0)
            Boundary::Error, // bits 15-14: 11 (value 3)
            Boundary::Error, // bits 13-12: 11 (value 3)
            Boundary::Error, // bits 11-10: 11 (value 3)
            Boundary::Undefined, // bits 9-8:   00 (value 0)
            Boundary::Undefined, // bits 7-6:   00 (value 0)
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_passages_invalid_values() {
        let data = [0xFF, 0xFF, 0xFF]; // Tous les bits à 1 (valeur 3)
        let passages = parse_passages(&data, 12);
        assert_eq!(passages.len(), 12);
        for passage in passages {
            assert_eq!(passage, Boundary::Error);
        }
    }

    #[test]
    fn test_parse_passages_specific_case() {
        // Exemple avec une séquence spécifique
        let data = [0b00011011, 0b01100110, 0b11001100];
        let passages = parse_passages(&data, 12);
        let expected = vec![
            Boundary::Undefined, // bits 23-22: 00 (value 0)
            Boundary::Open,      // bits 21-20: 01 (value 1)
            Boundary::Wall,      // bits 19-18: 10 (value 2)
            Boundary::Error,     // bits 17-16: 11 (value 3)
            Boundary::Open,      // bits 15-14: 01 (value 1)
            Boundary::Wall,      // bits 13-12: 10 (value 2)
            Boundary::Open,      // bits 11-10: 01 (value 1)
            Boundary::Wall,      // bits 9-8:   10 (value 2)
            Boundary::Error,     // bits 7-6:   11 (value 3)
            Boundary::Undefined, // bits 5-4:   00 (value 0)
            Boundary::Error,     // bits 3-2:   11 (value 3)
            Boundary::Undefined, // bits 1-0:   00 (value 0)
        ];
        assert_eq!(passages, expected);
    }

    #[test]
    fn test_parse_message_without_error() {
        let data = [0b00011010, 0b01100110, 0b10000100];
        let passages = parse_passages(&data, 12);
        let expected = vec![
            Boundary::Undefined, // bits 23-22: 00 (value 0)
            Boundary::Open,      // bits 21-20: 01 (value 1)
            Boundary::Wall,      // bits 19-18: 10 (value 2)
            Boundary::Wall,     // bits 17-16: 11 (value 3)
            Boundary::Open,      // bits 15-14: 01 (value 1)
            Boundary::Wall,      // bits 13-12: 10 (value 2)
            Boundary::Open,      // bits 11-10: 01 (value 1)
            Boundary::Wall,      // bits 9-8:   10 (value 2)
            Boundary::Wall,     // bits 7-6:   11 (value 3)
            Boundary::Undefined, // bits 5-4:   00 (value 0)
            Boundary::Open,     // bits 3-2:   11 (value 3)
            Boundary::Undefined, // bits 1-0:   00 (value 0)
        ];
    }
}

