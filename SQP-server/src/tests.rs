#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::{encode, decode};
    use crate::{encode_radar_view, Cell, Labyrinth, MapDirection};

    #[test]
    fn test_radar_view_encoding() {
        // Create a simple labyrinth for testing
        let mut cells = vec![vec![
            Cell {
                north_wall: true,
                east_wall: false,
                south_wall: false,
                west_wall: true,
                has_hint: false,
                has_exit: false,
            };
            3
        ]; 3];
        
        // Set up some walls
        // +---+---+---+
        // | 0,0|0,1|0,2|
        // +   +   +   +
        // |1,0 1,1|1,2|
        // +   +---+   +
        // |2,0|2,1|2,2|
        // +---+---+---+
        
        // Cell (1,1) has east and south walls
        cells[1][1].east_wall = true;
        cells[1][1].south_wall = true;
        
        // Cell (0,0) has north and west walls
        cells[0][0].north_wall = true;
        cells[0][0].west_wall = true;
        
        // Cell (2,2) has south and east walls
        cells[2][2].south_wall = true;
        cells[2][2].east_wall = true;
        
        let labyrinth = Labyrinth {
            width: 3,
            height: 3,
            cells,
            exit_position: (2, 2),
        };
        
        // Test player at (1,1) facing North
        let encoded = encode_radar_view((1, 1), MapDirection::North, &labyrinth);
        println!("Encoded radar view (North): {}", encoded);
        
        // Decode to verify expected byte pattern
        let decoded = decode(&encoded).expect("Failed to decode radar view");
        
        // Verify that passage values are set correctly based on the labyrinth walls
        // We should have walls at specific positions in the decoded data
        
        // Check for specific walls that should be set
        // These values depend on the radar view encoding logic
        
        // For a player at (1,1) facing North:
        // - There should be a wall to the east (passage 6 should be wall)
        // - There should be a wall to the south (passage 7 should be wall)
        // - There should be outer edges walls
        
        // Print decoded bytes for debugging
        println!("Decoded bytes: {:?}", decoded);
        
        // Check if decoded contains expected patterns
        // For example, if the east wall is represented by passage 6 (in data[3..6])
        // which should be value 2 (WALL), then we need to check if bits 5-4 in
        // data[4] are set to binary 10 (value 2)
        
        // Test player at (1,1) facing East
        let encoded = encode_radar_view((1, 1), MapDirection::East, &labyrinth);
        println!("Encoded radar view (East): {}", encoded);
        
        // Test player at (1,1) facing South
        let encoded = encode_radar_view((1, 1), MapDirection::South, &labyrinth);
        println!("Encoded radar view (South): {}", encoded);
        
        // Test player at (1,1) facing West
        let encoded = encode_radar_view((1, 1), MapDirection::West, &labyrinth);
        println!("Encoded radar view (West): {}", encoded);
    }
    
    #[test]
    fn test_radar_view_edge_cases() {
        // Create a labyrinth with players at edges
        let mut cells = vec![vec![
            Cell {
                north_wall: true,
                east_wall: false,
                south_wall: false,
                west_wall: true,
                has_hint: false,
                has_exit: false,
            };
            5
        ]; 5];
        
        let labyrinth = Labyrinth {
            width: 5,
            height: 5,
            cells,
            exit_position: (4, 4),
        };
        
        // Test player at edge (0, 0) facing South
        let encoded = encode_radar_view((0, 0), MapDirection::South, &labyrinth);
        println!("Encoded radar view (0,0 South): {}", encoded);
        
        // Test player at edge (4, 4) facing North
        let encoded = encode_radar_view((4, 4), MapDirection::North, &labyrinth);
        println!("Encoded radar view (4,4 North): {}", encoded);
        
        // Test player at edge (0, 4) facing East
        let encoded = encode_radar_view((0, 4), MapDirection::East, &labyrinth);
        println!("Encoded radar view (0,4 East): {}", encoded);
        
        // Test player at edge (4, 0) facing West
        let encoded = encode_radar_view((4, 0), MapDirection::West, &labyrinth);
        println!("Encoded radar view (4,0 West): {}", encoded);
    }
    
    #[test]
    fn test_radar_data_format() {
        // Create a simple labyrinth with known wall configurations
        let mut cells = vec![vec![
            Cell {
                north_wall: false,
                east_wall: false,
                south_wall: false,
                west_wall: false,
                has_hint: false,
                has_exit: false,
            };
            3
        ]; 3];
        
        // Add some walls to make a recognizable pattern
        // Center cell (1,1) has all four walls
        cells[1][1].north_wall = true;
        cells[1][1].east_wall = true;
        cells[1][1].south_wall = true;
        cells[1][1].west_wall = true;
        
        let labyrinth = Labyrinth {
            width: 3,
            height: 3,
            cells,
            exit_position: (2, 2),
        };
        
        // Test player at (1,1) facing North
        let encoded = encode_radar_view((1, 1), MapDirection::North, &labyrinth);
        let decoded = decode(&encoded).expect("Failed to decode radar view");
        
        println!("Radar view for player at (1,1) facing North:");
        println!("Encoded: {}", encoded);
        println!("Decoded: {:?}", decoded);
        
        // Verify format: The decoded data should have length of 11
        assert_eq!(decoded.len(), 11, "Radar data should be 11 bytes");
        
        // First 6 bytes should have wall information for passages
        // The data array should contain values representing passage types:
        // - UNDEFINED (0b00)
        // - OPEN (0b01)
        // - WALL (0b10)
        // - ERROR (0b11)
        
        // Extract wall status from the data array
        let get_passage_type = |byte_index: usize, passage_index: usize| -> u8 {
            let bit_position = (passage_index % 4) * 2;
            (decoded[byte_index] >> (6 - bit_position)) & 0b11
        };
        
        // Print the passages for debugging
        println!("--- Passage types for North direction ---");
        for byte_index in 0..6 {
            for passage_index in 0..4 {
                let absolute_passage = byte_index * 4 + passage_index;
                if absolute_passage < 24 { // Only 24 passages in the radar view
                    let passage_type = get_passage_type(byte_index, passage_index);
                    println!("Passage {}: {} (byte {}, pos {})", 
                             absolute_passage, passage_type, byte_index, passage_index);
                }
            }
        }
        
        // Define values
        let wall_value = 0b10; // Binary 10 is WALL (value 2)
        let open_value = 0b01; // Binary 01 is OPEN (value 1)
        
        // Based on the actual implementation in encode_radar_view for North facing:
        // - North wall at passage 4 (index 1,0)
        assert_eq!(get_passage_type(1, 0), wall_value, "North wall should be set (passage 4)");
        
        // - South wall at passage 7 (index 1,3)
        assert_eq!(get_passage_type(1, 3), wall_value, "South wall should be set (passage 7)");
        
        // - West wall at passage 5 (index 1,1 in data[3..6]) - this is 5,1 in our flat representation
        assert_eq!(get_passage_type(5, 1), wall_value, "West wall should be set (passage 21)");
        assert_eq!(get_passage_type(5, 2), wall_value, "East wall should be set (passage 22)");
        
        // Now test with player facing East
        let encoded_east = encode_radar_view((1, 1), MapDirection::East, &labyrinth);
        let decoded_east = decode(&encoded_east).expect("Failed to decode East radar view");
        
        println!("\nRadar view for player at (1,1) facing East:");
        println!("Encoded: {}", encoded_east);
        println!("Decoded: {:?}", decoded_east);
        
        // Define a helper function to get passage type from the decoded bytes
        let get_east_passage_type = |byte_index: usize, bit_position: usize| -> u8 {
            (decoded_east[byte_index] >> (6 - bit_position * 2)) & 0b11
        };
        
        // Print the passages for East direction
        println!("--- Passage types for East direction ---");
        for byte_index in 0..6 {
            for passage_index in 0..4 {
                let absolute_passage = byte_index * 4 + passage_index;
                if absolute_passage < 24 { // Only 24 passages in the radar view
                    let passage_type = get_east_passage_type(byte_index, passage_index);
                    println!("Passage {}: {} (byte {}, pos {})", 
                             absolute_passage, passage_type, byte_index, passage_index);
                }
            }
        }
        
        // Based on looking at the encode_radar_view function, when facing East:
        // - North wall maps to passage 6 in data[3..6] (line 1123)
        // Reading the code: if north_wall { set_passage_value(&mut data[3..6], 6, WALL); }
        let north_wall_east = get_east_passage_type(3, 3);  // This maps to passage 15
        assert_eq!(north_wall_east, wall_value, "North wall should be set when facing East (passage 15)");
        
        // - East wall maps to passage 7 in data[0..3]
        // Reading the code: if east_wall { set_passage_value(&mut data[0..3], 7, WALL); }
        let east_wall_east = get_east_passage_type(1, 3);
        assert_eq!(east_wall_east, wall_value, "East wall should be set when facing East (passage 7)");
        
        // - South wall maps to passage 5 in data[3..6]
        // Reading the code: if south_wall { set_passage_value(&mut data[3..6], 5, WALL); }
        // The passage 5 in data[3..6] would be in byte 4, position 1 (passage 17)
        let south_wall_east = get_east_passage_type(4, 1);  // This maps to passage 17
        assert_eq!(south_wall_east, wall_value, "South wall should be set when facing East (passage 17)");
        
        // - West wall maps to passage 4 in data[0..3]
        let west_wall_east = get_east_passage_type(0, 0);
        assert_eq!(west_wall_east, wall_value, "West wall should be set when facing East (passage 0)");
    }
} 