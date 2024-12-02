use serde::{Deserialize, Serialize};

/**
 * The Direction enum represents the different directions the player can face.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Direction {
    Front,
    Back,
    Left,
    Right,
}

/**
 * The turn_right function turns the player to the right.
 *
 * @param current_direction: &Direction - The current direction of the player
 * @return Direction - The new direction after turning right
 */
fn turn_right(current_direction: &Direction) -> Direction {
    match current_direction {
        Direction::Front => Direction::Right,
        Direction::Right => Direction::Back,
        Direction::Back => Direction::Left,
        Direction::Left => Direction::Front,
    }
}

/**
 * The turn_left function turns the player to the left.
 *
 * @param current_direction: &Direction - The current direction of the player
 * @return Direction - The new direction after turning left
 */
pub(crate) fn turn_left(current_direction: &Direction) -> Direction {
    match current_direction {
        Direction::Front => Direction::Left,
        Direction::Left => Direction::Back,
        Direction::Back => Direction::Right,
        Direction::Right => Direction::Front,
    }
}

/**
 * The move_forward function moves the player forward.
 *
 * @param current_direction: &Direction - The current direction of the player
 * @return Direction - The new direction after moving forward
 */
impl PartialEq for &Direction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Direction::Front, Direction::Front) => true,
            (Direction::Right, Direction::Right) => true,
            (Direction::Back, Direction::Back) => true,
            (Direction::Left, Direction::Left) => true,
            _ => false,
        }
    }
}


