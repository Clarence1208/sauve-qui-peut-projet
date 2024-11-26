use crate::server_utils::{receive_message, send_message};
use crate::{Action, Direction, Message, SubscribePlayer};
use std::net::TcpStream;
use crate::decoder::decode;

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

        parse_radar_response(&action_response);

        // timeout 1/100 of a second
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Check for exit condition
        if action_response.contains("FoundExit") {
            println!("Player {} found the exit!", player_name);
            break; // Exit the loop
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
    }

    pub(crate) fn parse_radar_response(response: &str) {
        if response.contains("CannotPassThroughWall") {
            return;
        }

        // Extract the radar data from the response as a string
        // response format: {"RadarView":"aeQrajHOapap//a"}
        let radar_data = response
            .split("\":\"")
            .nth(1)
            .unwrap()
            .split("\"")
            .next()
            .unwrap();


        // Print the radar data
        println!("Radar data: {}", radar_data);

        // decode the radar data
        let decoded_radar_data = decode(&radar_data);

        // Print the decoded radar data
        println!("Decoded radar data: {:?}", decoded_radar_data);
    }
}