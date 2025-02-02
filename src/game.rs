use crossterm::style::Color;
use std::time::{Duration, Instant};

use crate::shapes::{color_from_index, generate_bag, SHAPES};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

/// Board cell: None if empty, Some(color) if occupied.
pub type Board = [[Option<Color>; BOARD_WIDTH]; BOARD_HEIGHT];

/// Holds piece orientation, position, and shape index.
#[derive(Clone)]
pub struct Tetromino {
    pub shape_idx: usize,   // index into SHAPES
    pub orientation: usize, // 0..3
    pub x: i32,
    pub y: i32,
}

impl Tetromino {
    pub fn cells(&self) -> &[[i32; 2]; 4] {
        &SHAPES[self.shape_idx].orientations[self.orientation]
    }

    pub fn positions(&self) -> Vec<(i32, i32)> {
        // Convert each [x,y] into a tuple
        self.cells()
            .iter()
            .map(|&xy| (self.x + xy[0], self.y + xy[1]))
            .collect()
    }

    pub fn rotate_cw(&mut self) {
        self.orientation = (self.orientation + 1) % 4;
    }

    pub fn rotate_ccw(&mut self) {
        self.orientation = (self.orientation + 3) % 4;
    }

    pub fn color(&self) -> Color {
        let idx = SHAPES[self.shape_idx].color_index;
        color_from_index(idx)
    }

    pub fn name(&self) -> char {
        SHAPES[self.shape_idx].name
    }
}

/// Main game state
pub struct Game {
    pub board: Board,
    pub current_piece: Tetromino,
    pub hold_piece: Option<Tetromino>,
    pub can_hold: bool,

    pub bag: Vec<usize>,
    pub next_bag: Vec<usize>,
    pub next_queue: Vec<usize>,

    pub last_tick: Instant,
    pub fall_interval: Duration,

    pub level: u32,
    pub score: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
}

impl Game {
    pub fn new() -> Self {
        let mut bag = generate_bag();
        let mut next_bag = generate_bag();
        let mut next_queue = vec![];

        // Pre-populate 3 upcoming
        while next_queue.len() < 3 {
            if bag.is_empty() {
                bag = next_bag;
                next_bag = generate_bag();
            }
            next_queue.push(bag.remove(0));
        }

        // Spawn current piece
        let shape_idx = next_queue.remove(0);
        let current_piece = Tetromino {
            shape_idx,
            orientation: 0,
            x: (BOARD_WIDTH / 2) as i32 - 2,
            y: 0,
        };

        Game {
            board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
            current_piece,
            hold_piece: None,
            can_hold: true,

            bag,
            next_bag,
            next_queue,

            last_tick: Instant::now(),
            fall_interval: Duration::from_millis(500),

            level: 0,
            score: 0,
            lines_cleared: 0,
            game_over: false,
        }
    }

    /// Return true if `piece`'s positions are all in-bounds and unoccupied.
    pub fn is_valid_position(&self, piece: &Tetromino) -> bool {
        for &(px, py) in piece.positions().iter() {
            if px < 0 || px >= BOARD_WIDTH as i32 || py < 0 || py >= BOARD_HEIGHT as i32 {
                return false;
            }
            if self.board[py as usize][px as usize].is_some() {
                return false;
            }
        }
        true
    }

    pub fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let old_x = self.current_piece.x;
        let old_y = self.current_piece.y;
        self.current_piece.x += dx;
        self.current_piece.y += dy;
        if !self.is_valid_position(&self.current_piece) {
            // revert
            self.current_piece.x = old_x;
            self.current_piece.y = old_y;
            return false;
        }
        true
    }

    pub fn try_rotate_cw(&mut self) {
        let old_orientation = self.current_piece.orientation;
        self.current_piece.rotate_cw();
        if !self.is_valid_position(&self.current_piece) {
            // optional small wallkick attempt
            if !self.do_wallkick() {
                self.current_piece.orientation = old_orientation;
            }
        }
    }

    pub fn try_rotate_ccw(&mut self) {
        let old_orientation = self.current_piece.orientation;
        self.current_piece.rotate_ccw();
        if !self.is_valid_position(&self.current_piece) {
            if !self.do_wallkick() {
                self.current_piece.orientation = old_orientation;
            }
        }
    }

    /// Very naive wallkick that tries shifting piece left/right/up a bit.
    fn do_wallkick(&mut self) -> bool {
        let offsets = [(1, 0), (-1, 0), (2, 0), (-2, 0), (0, -1), (1, -1), (-1, -1)];
        let (orig_x, orig_y) = (self.current_piece.x, self.current_piece.y);
        for (ox, oy) in offsets {
            self.current_piece.x += ox;
            self.current_piece.y += oy;
            if self.is_valid_position(&self.current_piece) {
                return true;
            }
            self.current_piece.x = orig_x;
            self.current_piece.y = orig_y;
        }
        false
    }

    /// Lock the current piece into the board (cells become Some(color))
    pub fn lock_piece(&mut self) {
        for &(px, py) in self.current_piece.positions().iter() {
            if px >= 0 && px < BOARD_WIDTH as i32 && py >= 0 && py < BOARD_HEIGHT as i32 {
                self.board[py as usize][px as usize] = Some(self.current_piece.color());
            }
        }
    }

    /// Check for full lines, clear them, update score/level
    pub fn clear_lines(&mut self) {
        let mut lines_cleared_now = 0;
        for y in (0..BOARD_HEIGHT).rev() {
            let full = self.board[y].iter().all(|&cell| cell.is_some());
            if full {
                // shift everything down
                for row in (1..=y).rev() {
                    self.board[row] = self.board[row - 1];
                }
                self.board[0] = [None; BOARD_WIDTH];
                lines_cleared_now += 1;
            }
        }
        if lines_cleared_now > 0 {
            self.score += self.line_clear_score(lines_cleared_now);
            self.lines_cleared += lines_cleared_now as u32;

            // level up every 10 lines
            if (self.lines_cleared / 10) > self.level {
                self.level = self.lines_cleared / 10;
                // speed up
                let speed = 500_u64.saturating_sub(self.level as u64 * 40);
                self.fall_interval = Duration::from_millis(speed.max(100));
            }
        }
    }

    fn line_clear_score(&self, lines: usize) -> u32 {
        let base = match lines {
            1 => 40,
            2 => 100,
            3 => 300,
            4 => 1200,
            _ => 0,
        };
        base * (self.level + 1)
    }

    /// Spawn next piece from next_queue, refill queue as needed
    pub fn spawn_next_piece(&mut self) {
        while self.next_queue.len() < 3 {
            if self.bag.is_empty() {
                self.bag = self.next_bag.clone();
                self.next_bag = generate_bag();
            }
            self.next_queue.push(self.bag.remove(0));
        }
        let shape_idx = self.next_queue.remove(0);
        self.current_piece = Tetromino {
            shape_idx,
            orientation: 0,
            x: (BOARD_WIDTH / 2) as i32 - 2,
            y: 0,
        };
        self.can_hold = true;

        // Check immediate collision -> game over
        if !self.is_valid_position(&self.current_piece) {
            self.game_over = true;
        }
    }

    /// Swap current piece with hold piece (if allowed)
    pub fn hold_current_piece(&mut self) {
        if !self.can_hold {
            return;
        }
        self.can_hold = false;

        match &mut self.hold_piece {
            Some(hp) => {
                let old_shape_idx = self.current_piece.shape_idx;
                let old_orientation = self.current_piece.orientation;
                // move hold piece into current
                self.current_piece.shape_idx = hp.shape_idx;
                self.current_piece.orientation = hp.orientation;
                self.current_piece.x = (BOARD_WIDTH / 2) as i32 - 2;
                self.current_piece.y = 0;

                // store old in hold
                hp.shape_idx = old_shape_idx;
                hp.orientation = old_orientation;
            }
            None => {
                self.hold_piece = Some(self.current_piece.clone());
                self.spawn_next_piece();
            }
        }
    }

    /// Gravity update
    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= self.fall_interval {
            self.last_tick = Instant::now();
            if !self.try_move(0, 1) {
                // can't move down -> lock + spawn next
                self.lock_piece();
                self.clear_lines();
                self.spawn_next_piece();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_clear_scoring() {
        let game = Game::new();
        assert_eq!(game.score, 0);
        // awarding lines
        // artificially add lines_cleared for testing
        assert_eq!(game.line_clear_score(1), 40);
        assert_eq!(game.line_clear_score(2), 100);
        assert_eq!(game.line_clear_score(3), 300);
        assert_eq!(game.line_clear_score(4), 1200);
    }
}
