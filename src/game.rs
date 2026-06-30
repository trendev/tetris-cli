//! The Tetris state machine: board, active piece, scoring/levels, and the
//! lock cycle that ties them together.
//!
//! Coordinates throughout this module are `(x, y)` with the origin at the
//! top-left, `x` growing right and `y` growing down. The board is stored
//! row-major as `board[y][x]` (row first, then column) - easy to get backwards,
//! so watch for that when indexing directly instead of via `positions()`.
use crossterm::style::Color;
use std::time::{Duration, Instant};

use crate::shapes::{color_from_index, generate_bag, SHAPES};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

/// Board cell: None if empty, Some(color) if occupied.
pub type Board = [[Option<Color>; BOARD_WIDTH]; BOARD_HEIGHT];

/// A falling (or held) piece: which shape it is, which of its 4 rotations is
/// active, and its position on the board.
#[derive(Clone)]
pub struct Tetromino {
    /// Index into [`SHAPES`] selecting which of the 7 tetrominoes this is.
    pub shape_idx: usize,
    /// Which of the shape's 4 precomputed rotations is currently active.
    /// Rotating just changes this index (mod 4); see [`Tetromino::rotate_cw`].
    pub orientation: usize,
    /// Board-space x/y of this piece's local origin. Cell coordinates from
    /// [`SHAPES`] are added to this to get absolute board positions.
    pub x: i32,
    pub y: i32,
}

impl Tetromino {
    /// The 4 local `(x, y)` cell offsets for the current shape/orientation,
    /// as laid out in [`crate::shapes`].
    pub fn cells(&self) -> &[[i32; 2]; 4] {
        &SHAPES[self.shape_idx].orientations[self.orientation]
    }

    /// Absolute board positions occupied by this piece, i.e. each local cell
    /// offset added to the piece's `(x, y)`.
    pub fn positions(&self) -> Vec<(i32, i32)> {
        self.cells()
            .iter()
            .map(|&xy| (self.x + xy[0], self.y + xy[1]))
            .collect()
    }

    /// Rotate clockwise by stepping to the next precomputed orientation.
    /// Shapes have no rotation math at runtime - each orientation's cell
    /// layout is hardcoded in `shapes.rs`, so "rotating" is just indexing.
    pub fn rotate_cw(&mut self) {
        self.orientation = (self.orientation + 1) % 4;
    }

    /// Rotate counter-clockwise. Adding 3 (rather than subtracting 1) avoids
    /// underflow since `orientation` is unsigned.
    pub fn rotate_ccw(&mut self) {
        self.orientation = (self.orientation + 3) % 4;
    }

    /// The display color for this piece's shape, looked up via its
    /// `color_index` and converted to a [`Color`] for rendering.
    pub fn color(&self) -> Color {
        let idx = SHAPES[self.shape_idx].color_index;
        color_from_index(idx)
    }

    /// Single-letter shape name (I, O, T, S, Z, J, L) shown in the UI for
    /// the hold slot and next-piece queue.
    pub fn name(&self) -> char {
        SHAPES[self.shape_idx].name
    }
}

/// Main game state: the board, the piece currently falling, the hold slot,
/// the 7-bag randomizer state, and scoring/timing.
pub struct Game {
    pub board: Board,
    pub current_piece: Tetromino,
    /// The piece tucked away via the hold action (`C` key), if any.
    pub hold_piece: Option<Tetromino>,
    /// Whether holding is currently allowed. Disabled after a swap-hold so a
    /// player can't hold back and forth indefinitely to stall a piece - it's
    /// re-enabled the next time a piece locks and a new one spawns. See
    /// [`Game::hold_current_piece`] for the asymmetry between the first hold
    /// and subsequent swaps.
    pub can_hold: bool,

    /// Shape indices for the bag currently being drawn from.
    pub bag: Vec<usize>,
    /// The *next* full bag, pre-generated so a fresh 7-bag is always ready
    /// the instant `bag` runs dry - see [`generate_bag`] for the shuffle.
    pub next_bag: Vec<usize>,
    /// Look-ahead queue of upcoming shapes (kept topped up to 3) used both to
    /// spawn the next piece and to render the "Next" preview.
    pub next_queue: Vec<usize>,

    /// When the piece last fell one row under gravity.
    pub last_tick: Instant,
    /// How long between automatic gravity drops; shrinks as the level rises.
    pub fall_interval: Duration,

    pub level: u32,
    pub score: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
}

impl Game {
    pub fn new() -> Self {
        // Two bags are kept on hand: `bag` is drawn from for new pieces, and
        // `next_bag` is generated immediately so `spawn_next_piece` can swap
        // it in mid-queue-fill without ever blocking on a fresh shuffle.
        let mut bag = generate_bag();
        let mut next_bag = generate_bag();
        let mut next_queue = vec![];

        // Pre-populate the look-ahead to 3 pieces, refilling `bag` from
        // `next_bag` if it runs out partway through.
        while next_queue.len() < 3 {
            if bag.is_empty() {
                bag = next_bag;
                next_bag = generate_bag();
            }
            next_queue.push(bag.remove(0));
        }

        // Consume one piece from the queue to spawn the active piece, so
        // `next_queue` ends up holding 2 (not 3) right after construction.
        let shape_idx = next_queue.remove(0);
        let current_piece = Tetromino {
            shape_idx,
            orientation: 0,
            // Centered-ish spawn column; -2 accounts for shapes being defined
            // within a 4-wide local grid (see `shapes.rs`).
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
    /// This is the single source of truth for "can the piece be here?" -
    /// movement, rotation, wallkicks, and spawning all funnel through it.
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

    /// Attempt to shift the current piece by `(dx, dy)`. Applies the move
    /// speculatively, then rolls it back if that lands on an invalid
    /// position (out of bounds or overlapping a locked cell). Returns
    /// whether the move succeeded, which callers use to detect "can't go
    /// any further" (e.g. soft drop locking the piece on failure).
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

    /// Rotate clockwise, falling back to a wallkick and finally reverting if
    /// the new orientation doesn't fit anywhere.
    pub fn try_rotate_cw(&mut self) {
        let old_orientation = self.current_piece.orientation;
        self.current_piece.rotate_cw();
        // On collision, attempt a small wallkick; revert if that also fails.
        if !self.is_valid_position(&self.current_piece) && !self.do_wallkick() {
            self.current_piece.orientation = old_orientation;
        }
    }

    /// Same as [`Game::try_rotate_cw`] but counter-clockwise.
    pub fn try_rotate_ccw(&mut self) {
        let old_orientation = self.current_piece.orientation;
        self.current_piece.rotate_ccw();
        if !self.is_valid_position(&self.current_piece) && !self.do_wallkick() {
            self.current_piece.orientation = old_orientation;
        }
    }

    /// Very naive wallkick: try a handful of fixed offsets (left/right by 1
    /// or 2, up by 1, and the diagonals) and keep the first one that makes
    /// the post-rotation position valid. This is *not* the official SRS
    /// kick-table system (which picks offsets per-shape and per-rotation) -
    /// it's a small fixed list that's good enough to let rotations succeed
    /// near walls/floor without implementing full SRS.
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

    /// Stamp the current piece's cells into the board as permanent, colored
    /// cells. Called once the piece can no longer fall (see
    /// [`Game::lock_and_advance`]); positions are already guaranteed valid at
    /// this point, the bounds check here is just defensive.
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
        // Keep only rows with at least one empty cell; full rows are dropped.
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

        // Surviving rows fall to the bottom; empty rows fill the top. Writing
        // `kept` rows starting at index `lines_cleared_now` (rather than 0)
        // is what makes everything above the gaps "fall" down by exactly the
        // number of lines removed, regardless of whether those lines were
        // adjacent.
        let mut board = [[None; BOARD_WIDTH]; BOARD_HEIGHT];
        for (i, row) in kept.into_iter().enumerate() {
            board[lines_cleared_now + i] = row;
        }
        self.board = board;

        // Score using the level in effect *before* any level-up below, so
        // e.g. clearing the 10th line still scores at the old (slower) level
        // and only the level counter advances afterwards.
        self.score += self.line_clear_score(lines_cleared_now);
        self.lines_cleared += lines_cleared_now as u32;

        // Level up every 10 total lines cleared, and speed up gravity to
        // match - each level shaves 40ms off the fall interval, floored at
        // 100ms so the game never becomes literally unplayable.
        if (self.lines_cleared / 10) > self.level {
            self.level = self.lines_cleared / 10;
            let speed = 500_u64.saturating_sub(self.level as u64 * 40);
            self.fall_interval = Duration::from_millis(speed.max(100));
        }
    }

    /// Classic Tetris scoring table (single/double/triple/tetris), scaled by
    /// `level + 1` so clears are worth more at higher levels.
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

    /// Pull the next shape off `next_queue` to become the active piece,
    /// topping the queue back up to 3 from the bag (refilling the bag from
    /// `next_bag` if needed - see [`Game::new`] for why two bags exist).
    /// Re-enables holding for the new piece, and ends the game if the new
    /// piece can't even fit at its spawn position (board topped out).
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

    /// Hold (`C` key): stash the current piece for later and bring back
    /// whatever was previously held, or - on the very first hold - just
    /// stash the current piece and spawn a fresh one from the queue.
    ///
    /// The two branches intentionally behave differently around `can_hold`:
    /// the **swap** branch (`Some`) sets `can_hold = false` and leaves it
    /// there, preventing the player from holding repeatedly to stall a
    /// piece indefinitely. The **first hold** (`None`) branch instead calls
    /// [`Game::spawn_next_piece`], which itself resets `can_hold = true` -
    /// so the very first hold does *not* lock out a second hold right away;
    /// the lockout only kicks in once a swap has actually happened.
    pub fn hold_current_piece(&mut self) {
        if !self.can_hold {
            return;
        }
        self.can_hold = false;

        match &mut self.hold_piece {
            Some(hp) => {
                let old_shape_idx = self.current_piece.shape_idx;
                let old_orientation = self.current_piece.orientation;
                // Move the held piece into play, respawning it at the
                // standard spawn position (not wherever the current piece
                // happened to be).
                self.current_piece.shape_idx = hp.shape_idx;
                self.current_piece.orientation = hp.orientation;
                self.current_piece.x = (BOARD_WIDTH / 2) as i32 - 2;
                self.current_piece.y = 0;

                // Stash what was falling into the hold slot.
                hp.shape_idx = old_shape_idx;
                hp.orientation = old_orientation;
            }
            None => {
                self.hold_piece = Some(self.current_piece.clone());
                self.spawn_next_piece();
            }
        }
    }

    /// The lock cycle: stamp the piece into the board, clear any now-full
    /// rows, then spawn the next piece. Both gravity (`soft_drop`/`update`)
    /// and a manual hard drop funnel through this so the sequence is never
    /// duplicated or reordered.
    pub fn lock_and_advance(&mut self) {
        self.lock_piece();
        self.clear_lines();
        self.spawn_next_piece();
    }

    /// Move the piece down one row; if it can't fall any further, treat that
    /// as a lock (this is also what gravity calls every tick).
    pub fn soft_drop(&mut self) {
        if !self.try_move(0, 1) {
            self.lock_and_advance();
        }
    }

    /// Drop the piece straight down as far as it will go, then lock it
    /// immediately - skipping the usual per-row gravity ticks.
    pub fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_and_advance();
    }

    /// Called once per main-loop iteration. Applies exactly one soft drop's
    /// worth of gravity once `fall_interval` has elapsed since the last
    /// tick; otherwise a no-op. Resets the tick clock regardless of how long
    /// the loop took, so timing tracks wall-clock time rather than frames.
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
