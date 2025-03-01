use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone)]
pub struct Cell {
    pub north_wall: bool,
    pub east_wall: bool,
    pub south_wall: bool,
    pub west_wall: bool,
    pub has_hint: bool,
    pub has_exit: bool,
    pub visited: bool, // Used during generation
}

impl Cell {
    fn new() -> Self {
        Cell {
            north_wall: true,
            east_wall: true,
            south_wall: true,
            west_wall: true,
            has_hint: false,
            has_exit: false,
            visited: false,
        }
    }
}

pub struct Maze {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
    pub exit_position: (usize, usize),
}

// Directions used for maze generation
#[derive(Clone, Copy, Debug)]
enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }
}

/// Generate a maze using the Recursive Backtracking algorithm
/// This ensures all cells are reachable and there are no isolated sections
pub fn generate_maze(width: usize, height: usize) -> Maze {
    let mut rng = rand::thread_rng();

    // Initialize cells with all walls
    let mut cells = vec![vec![Cell::new(); width]; height];

    // Start at a random position
    let start_x = rng.gen_range(0..width);
    let start_y = rng.gen_range(0..height);

    // Stack for recursive backtracking
    let mut stack = Vec::new();
    cells[start_y][start_x].visited = true;
    stack.push((start_x, start_y));

    while !stack.is_empty() {
        let (current_x, current_y) = *stack.last().unwrap();

        let mut neighbors = Vec::new();

        // Check North
        if current_y > 0 && !cells[current_y - 1][current_x].visited {
            neighbors.push((current_x, current_y - 1, Direction::North));
        }

        // Check East
        if current_x < width - 1 && !cells[current_y][current_x + 1].visited {
            neighbors.push((current_x + 1, current_y, Direction::East));
        }

        // Check South
        if current_y < height - 1 && !cells[current_y + 1][current_x].visited {
            neighbors.push((current_x, current_y + 1, Direction::South));
        }

        // Check West
        if current_x > 0 && !cells[current_y][current_x - 1].visited {
            neighbors.push((current_x - 1, current_y, Direction::West));
        }

        if !neighbors.is_empty() {
            // Choose a random unvisited neighbor
            let (next_x, next_y, direction) = neighbors.choose(&mut rng).unwrap().clone();

            // Remove the wall between current cell and chosen cell
            match direction {
                Direction::North => {
                    cells[current_y][current_x].north_wall = false;
                    cells[next_y][next_x].south_wall = false;
                }
                Direction::East => {
                    cells[current_y][current_x].east_wall = false;
                    cells[next_y][next_x].west_wall = false;
                }
                Direction::South => {
                    cells[current_y][current_x].south_wall = false;
                    cells[next_y][next_x].north_wall = false;
                }
                Direction::West => {
                    cells[current_y][current_x].west_wall = false;
                    cells[next_y][next_x].east_wall = false;
                }
            }

            // Mark the new cell as visited and push it to the stack
            cells[next_y][next_x].visited = true;
            stack.push((next_x, next_y));
        } else {
            // No unvisited neighbors, backtrack
            stack.pop();
        }
    }

    // Place the exit at a position far from the start
    let (exit_x, exit_y) = find_farthest_point(&cells, start_x, start_y, width, height);
    cells[exit_y][exit_x].has_exit = true;

    // Place hints
    place_hints(&mut cells, width, height, (exit_x, exit_y));

    // Remove the 'visited' flag for all cells
    for row in &mut cells {
        for cell in row {
            cell.visited = false;
        }
    }

    Maze {
        width,
        height,
        cells,
        exit_position: (exit_x, exit_y),
    }
}

/// Find the point farthest from the start
fn find_farthest_point(
    cells: &Vec<Vec<Cell>>,
    start_x: usize,
    start_y: usize,
    width: usize,
    height: usize,
) -> (usize, usize) {
    let mut distances = vec![vec![None; width]; height];
    let mut queue = std::collections::VecDeque::new();

    // Start with the initial position
    distances[start_y][start_x] = Some(0);
    queue.push_back((start_x, start_y, 0));

    let mut farthest_point = (start_x, start_y);
    let mut max_distance = 0;

    while let Some((x, y, distance)) = queue.pop_front() {
        // Update the farthest point if needed
        if distance > max_distance {
            max_distance = distance;
            farthest_point = (x, y);
        }

        // Check North
        if y > 0 && !cells[y][x].north_wall && distances[y - 1][x].is_none() {
            distances[y - 1][x] = Some(distance + 1);
            queue.push_back((x, y - 1, distance + 1));
        }

        // Check East
        if x < width - 1 && !cells[y][x].east_wall && distances[y][x + 1].is_none() {
            distances[y][x + 1] = Some(distance + 1);
            queue.push_back((x + 1, y, distance + 1));
        }

        // Check South
        if y < height - 1 && !cells[y][x].south_wall && distances[y + 1][x].is_none() {
            distances[y + 1][x] = Some(distance + 1);
            queue.push_back((x, y + 1, distance + 1));
        }

        // Check West
        if x > 0 && !cells[y][x].west_wall && distances[y][x - 1].is_none() {
            distances[y][x - 1] = Some(distance + 1);
            queue.push_back((x - 1, y, distance + 1));
        }
    }

    farthest_point
}

/// Place hints in the maze to guide players toward the exit
fn place_hints(cells: &mut Vec<Vec<Cell>>, width: usize, height: usize, exit_pos: (usize, usize)) {
    let mut rng = rand::thread_rng();
    let num_hints = (width.min(height) / 2).max(1);

    for _ in 0..num_hints {
        let mut hint_x;
        let mut hint_y;

        // Ensure we don't place a hint at the exit
        loop {
            hint_x = rng.gen_range(0..width);
            hint_y = rng.gen_range(0..height);

            if (hint_x != exit_pos.0 || hint_y != exit_pos.1) && !cells[hint_y][hint_x].has_hint {
                break;
            }
        }

        cells[hint_y][hint_x].has_hint = true;
    }
}
