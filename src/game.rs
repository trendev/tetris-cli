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
        // On collision, attempt a small wallkick; revert if that also fails.
        if !self.is_valid_position(&self.current_piece) && !self.do_wallkick() {
            self.current_piece.orientation = old_orientation;
        }
    }

    pub fn try_rotate_ccw(&mut self) {
        let old_orientation = self.current_piece.orientation;
        self.current_piece.rotate_ccw();
        if !self.is_valid_position(&self.current_piece) && !self.do_wallkick() {
            self.current_piece.orientation = old_orientation;
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

    /// Check for full lines, clear them, update score/level.
    ///
    /// Rebuilds the board from the rows that are not full and pushes them to the bottom,
    /// which correctly compacts any number of cleared lines (including non-adjacent ones).
    pub fn clear_lines(&mut self) {
        let kept: Vec<[Option<Color>; BOARD_WIDTH]> = self
            .board
            .iter()
            .filter(|row| row.iter().any(|cell| cell.is_none()))
            .copied()
            .collect();

        let lines_cleared_now = BOARD_HEIGHT - kept.len();
        if lines_cleared_now == 0 {
            return;
        }

        // Surviving rows fall to the bottom; empty rows fill the top.
        let mut board = [[None; BOARD_WIDTH]; BOARD_HEIGHT];
        for (i, row) in kept.into_iter().enumerate() {
            board[lines_cleared_now + i] = row;
        }
        self.board = board;

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

    /// Lock the current piece, clear any completed lines, and spawn the next piece.
    pub fn lock_and_advance(&mut self) {
        self.lock_piece();
        self.clear_lines();
        self.spawn_next_piece();
    }

    /// Move the piece down one row; if it can't fall, lock it and advance.
    pub fn soft_drop(&mut self) {
        if !self.try_move(0, 1) {
            self.lock_and_advance();
        }
    }

    /// Drop the piece as far as it can go, then lock it and advance.
    pub fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_and_advance();
    }

    /// Gravity update: apply a soft drop once the fall interval has elapsed.
    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= self.fall_interval {
            self.last_tick = Instant::now();
            self.soft_drop();
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a piece of the given shape at a fixed position for deterministic tests.
    fn piece_at(shape_idx: usize, x: i32, y: i32) -> Tetromino {
        Tetromino {
            shape_idx,
            orientation: 0,
            x,
            y,
        }
    }

    /// Fill an entire board row so it is ready to be cleared.
    fn fill_row(game: &mut Game, y: usize) {
        for x in 0..BOARD_WIDTH {
            game.board[y][x] = Some(Color::White);
        }
    }

    #[test]
    fn test_new_and_default_match_shape() {
        let game = Game::new();
        assert!(!game.game_over);
        // new() fills the look-ahead to 3 then removes one to spawn the current piece.
        assert_eq!(game.next_queue.len(), 2);
        assert!(game.board.iter().flatten().all(|c| c.is_none()));

        let default_game = Game::default();
        assert_eq!(default_game.next_queue.len(), 2);
        assert!(!default_game.game_over);
    }

    #[test]
    fn test_tetromino_positions_and_metadata() {
        // I-piece (shape 0) orientation 0 occupies row y=1 across four columns.
        let piece = piece_at(0, 0, 0);
        let mut positions = piece.positions();
        positions.sort();
        assert_eq!(positions, vec![(0, 1), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(piece.color(), Color::Cyan);
        assert_eq!(piece.name(), 'I');
    }

    #[test]
    fn test_rotation_wraps() {
        let mut piece = piece_at(2, 0, 0); // T-piece
        for expected in [1, 2, 3, 0] {
            piece.rotate_cw();
            assert_eq!(piece.orientation, expected);
        }
        for expected in [3, 2, 1, 0] {
            piece.rotate_ccw();
            assert_eq!(piece.orientation, expected);
        }
    }

    #[test]
    fn test_is_valid_position_bounds_and_occupancy() {
        let mut game = Game::new();
        // Out of bounds on each side.
        assert!(!game.is_valid_position(&piece_at(0, -5, 0))); // left
        assert!(!game.is_valid_position(&piece_at(0, BOARD_WIDTH as i32, 0))); // right
        assert!(!game.is_valid_position(&piece_at(0, 0, BOARD_HEIGHT as i32))); // bottom
        assert!(!game.is_valid_position(&piece_at(0, 0, -5))); // top
                                                               // Valid placement near the top.
        assert!(game.is_valid_position(&piece_at(0, 0, 0)));
        // Occupied cell makes it invalid.
        game.board[1][0] = Some(Color::Red);
        assert!(!game.is_valid_position(&piece_at(0, 0, 0)));
    }

    #[test]
    fn test_try_move_success_and_revert() {
        let mut game = Game::new();
        let (x, y) = (game.current_piece.x, game.current_piece.y);
        assert!(game.try_move(0, 1));
        assert_eq!((game.current_piece.x, game.current_piece.y), (x, y + 1));

        // Moving far left must fail and revert position.
        let before = (game.current_piece.x, game.current_piece.y);
        assert!(!game.try_move(-100, 0));
        assert_eq!((game.current_piece.x, game.current_piece.y), before);
    }

    #[test]
    fn test_rotate_reverts_when_blocked() {
        let mut game = Game::new();
        // Fill the whole board except exactly the four cells the I-piece occupies in
        // orientation 0. Any rotation (and every wallkick offset) then collides, so the
        // orientation must revert to its original value.
        let piece = piece_at(0, 0, 0);
        let free: Vec<(i32, i32)> = piece.positions();
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                if !free.contains(&(x as i32, y as i32)) {
                    game.board[y][x] = Some(Color::White);
                }
            }
        }
        game.current_piece = piece;
        assert!(game.is_valid_position(&game.current_piece));
        game.try_rotate_cw();
        assert_eq!(game.current_piece.orientation, 0);
        game.try_rotate_ccw();
        assert_eq!(game.current_piece.orientation, 0);
    }

    #[test]
    fn test_rotate_with_wallkick() {
        let mut game = Game::new();
        // I-piece against the right wall: rotating needs a wallkick to fit.
        game.current_piece = piece_at(0, BOARD_WIDTH as i32 - 2, 0);
        game.try_rotate_cw();
        // Whatever the outcome, the resulting position must be valid.
        assert!(game.is_valid_position(&game.current_piece));
    }

    #[test]
    fn test_lock_piece_writes_colors() {
        let mut game = Game::new();
        game.current_piece = piece_at(0, 0, 0);
        let color = game.current_piece.color();
        game.lock_piece();
        for (x, y) in [(0, 1), (1, 1), (2, 1), (3, 1)] {
            assert_eq!(game.board[y][x], Some(color));
        }
    }

    #[test]
    fn test_clear_single_line_scoring_and_count() {
        let mut game = Game::new();
        fill_row(&mut game, BOARD_HEIGHT - 1);
        game.clear_lines();
        assert_eq!(game.lines_cleared, 1);
        assert_eq!(game.score, 40);
        assert!(game.board[BOARD_HEIGHT - 1].iter().all(|c| c.is_none()));
    }

    #[test]
    fn test_clear_tetris_and_non_adjacent() {
        let mut game = Game::new();
        // Four full rows at the bottom -> a "tetris".
        for y in (BOARD_HEIGHT - 4)..BOARD_HEIGHT {
            fill_row(&mut game, y);
        }
        game.clear_lines();
        assert_eq!(game.lines_cleared, 4);
        assert_eq!(game.score, 1200);

        // Two non-adjacent full rows clear together and count as two lines.
        let mut game2 = Game::new();
        fill_row(&mut game2, 5);
        fill_row(&mut game2, 10);
        game2.clear_lines();
        assert_eq!(game2.lines_cleared, 2);
    }

    #[test]
    fn test_clear_none_leaves_state() {
        let mut game = Game::new();
        // A partially filled row is not cleared.
        for x in 0..(BOARD_WIDTH - 1) {
            game.board[BOARD_HEIGHT - 1][x] = Some(Color::White);
        }
        game.clear_lines();
        assert_eq!(game.lines_cleared, 0);
        assert_eq!(game.score, 0);
    }

    #[test]
    fn test_level_up_speeds_up_fall_interval() {
        let mut game = Game::new();
        let initial = game.fall_interval;
        // Simulate having already cleared 9 lines, then clear one full row to reach 10.
        game.lines_cleared = 9;
        fill_row(&mut game, BOARD_HEIGHT - 1);
        game.clear_lines();
        assert_eq!(game.level, 1);
        assert!(game.fall_interval < initial);
        // The line is scored at the level in effect *before* the level-up (40 * (0 + 1)).
        assert_eq!(game.score, 40);
    }

    #[test]
    fn test_line_clear_scoring_scales_with_level() {
        let mut game = Game::new();
        assert_eq!(game.line_clear_score(1), 40);
        assert_eq!(game.line_clear_score(2), 100);
        assert_eq!(game.line_clear_score(3), 300);
        assert_eq!(game.line_clear_score(4), 1200);
        assert_eq!(game.line_clear_score(0), 0);
        game.level = 2;
        assert_eq!(game.line_clear_score(1), 40 * 3);
    }

    #[test]
    fn test_spawn_next_piece_refills_queue() {
        let mut game = Game::new();
        game.next_queue.clear();
        game.spawn_next_piece();
        // Queue is refilled to three then one is consumed to spawn, leaving two look-ahead.
        assert_eq!(game.next_queue.len(), 2);
        assert!(game.can_hold);
    }

    #[test]
    fn test_hold_gate_and_branches() {
        let mut game = Game::new();
        // can_hold == false short-circuits.
        game.can_hold = false;
        let before = game.current_piece.shape_idx;
        game.hold_current_piece();
        assert_eq!(game.current_piece.shape_idx, before);
        assert!(game.hold_piece.is_none());

        // None branch: stores the current piece into hold and spawns the next piece.
        // (Spawning re-enables can_hold, so the first hold doesn't lock holding out.)
        game.can_hold = true;
        let first = game.current_piece.shape_idx;
        game.hold_current_piece();
        assert_eq!(game.hold_piece.as_ref().unwrap().shape_idx, first);
        assert!(game.can_hold);

        // Some branch: swaps current and hold, and disables hold until the next lock.
        game.can_hold = true;
        let current = game.current_piece.shape_idx;
        game.hold_current_piece();
        assert_eq!(game.current_piece.shape_idx, first);
        assert_eq!(game.hold_piece.as_ref().unwrap().shape_idx, current);
        assert!(!game.can_hold);
    }

    #[test]
    fn test_soft_and_hard_drop() {
        let mut game = Game::new();
        let y = game.current_piece.y;
        game.soft_drop();
        assert_eq!(game.current_piece.y, y + 1);

        let mut game2 = Game::new();
        game2.hard_drop();
        let filled = game2.board.iter().flatten().filter(|c| c.is_some()).count();
        assert_eq!(filled, 4);
    }

    #[test]
    fn test_update_applies_gravity_when_interval_elapsed() {
        let mut game = Game::new();
        // Force the fall interval to have elapsed.
        game.fall_interval = Duration::from_millis(0);
        let y = game.current_piece.y;
        game.update();
        assert_eq!(game.current_piece.y, y + 1);
    }

    #[test]
    fn test_update_noop_before_interval() {
        let mut game = Game::new();
        // A very large interval means no gravity is applied this tick.
        game.fall_interval = Duration::from_secs(3600);
        game.last_tick = Instant::now();
        let y = game.current_piece.y;
        game.update();
        assert_eq!(game.current_piece.y, y);
    }
}
