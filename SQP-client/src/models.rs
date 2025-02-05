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
        matches!(
            (self, other),
            (Direction::Front, Direction::Front)
                | (Direction::Right, Direction::Right)
                | (Direction::Back, Direction::Back)
                | (Direction::Left, Direction::Left)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_turn_right() {
        // Turning right from Front should yield Right.
        assert_eq!(&turn_right(&Direction::Front), &Direction::Right);
        // Turning right from Right should yield Back.
        assert_eq!(&turn_right(&Direction::Right), &Direction::Back);
        // Turning right from Back should yield Left.
        assert_eq!(&turn_right(&Direction::Back), &Direction::Left);
        // Turning right from Left should yield Front.
        assert_eq!(&turn_right(&Direction::Left), &Direction::Front);
    }

    #[test]
    fn test_turn_left() {
        // Turning left from Front should yield Left.
        assert_eq!(&turn_left(&Direction::Front), &Direction::Left);
        // Turning left from Left should yield Back.
        assert_eq!(&turn_left(&Direction::Left), &Direction::Back);
        // Turning left from Back should yield Right.
        assert_eq!(&turn_left(&Direction::Back), &Direction::Right);
        // Turning left from Right should yield Front.
        assert_eq!(&turn_left(&Direction::Right), &Direction::Front);
    }

    #[test]
    fn test_partial_eq() {
        let front_a = &Direction::Front;
        let front_b = &Direction::Front;
        let left = &Direction::Left;
        // Two references to Front should be equal.
        assert_eq!(front_a, front_b);
        // Front and Left should not be equal.
        assert_ne!(front_a, left);
    }

    #[test]
    fn test_serde_serialization() {
        // Test that a Direction can be serialized to JSON and deserialized back.
        let direction = Direction::Back;
        let json = serde_json::to_string(&direction).unwrap();
        let deserialized: Direction = serde_json::from_str(&json).unwrap();
        assert_eq!(&deserialized, &direction);
    }
}
