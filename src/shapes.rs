//! The 7 tetromino shapes and the 7-bag randomizer.
//!
//! Each shape's 4 rotations are hardcoded cell lists rather than computed
//! with rotation math at runtime - see the diagrams below each `_ORIENTATIONS`
//! const for what each one looks like. All coordinates are local `(x, y)`
//! offsets within a 4x4 cell grid (the largest space any piece needs), with
//! `x` as the column and `y` as the row, both starting at 0 in the top-left.
//! These offsets get added to a `Tetromino`'s board position in
//! `Tetromino::positions` (see `game.rs`).
use rand::{rng, seq::SliceRandom};

/// Identifies each shape's display color. Stored as a plain `u8` on
/// `TetrominoShape` (rather than this enum directly) so the shape table
/// below can be a `const` - the `crossterm::Color` it maps to is only
/// resolved at render time via [`color_from_index`].
#[derive(Copy, Clone, Debug)]
pub enum ShapeColor {
    Cyan = 0,
    Yellow,
    Magenta,
    Green,
    Red,
    Blue,
    DarkYellow,
}

/// One shape's 4 orientations. Each orientation has 4 cells,
/// and each cell is a pair of i32 coordinates: (x,y).
type Orientations = [[[i32; 2]; 4]; 4];

/// The data for a single tetromino: 4 orientations, color, name.
#[derive(Copy, Clone)]
pub struct TetrominoShape {
    pub orientations: Orientations,
    pub color_index: u8, // map to crossterm::Color in code
    pub name: char,
}

//
// Define each shape's 4 orientations as a 4x4 array of (x,y).
//
// Each constant below is annotated with a small diagram of *orientation 0
// only* (`X` = occupied cell, `.` = empty, reading left-to-right/top-to-
// bottom like the (x, y) coordinates themselves). Orientations 1-3 are the
// same piece pre-rotated 90/180/270 degrees clockwise; they aren't derived
// from orientation 0 at runtime; they're just written out by hand below in
// the same style, so the diagram is there as a key to read the numbers, not
// a complete picture of every rotation.
//
// I-piece, orientation 0 - a horizontal 4-in-a-row:
// . . . .
// X X X X
const I_ORIENTATIONS: Orientations = [
    // orientation 0
    [[0, 1], [1, 1], [2, 1], [3, 1]],
    // orientation 1 (vertical)
    [[2, 0], [2, 1], [2, 2], [2, 3]],
    // orientation 2 (horizontal again, shifted down a row vs. orientation 0)
    [[0, 2], [1, 2], [2, 2], [3, 2]],
    // orientation 3 (vertical again, shifted left a column vs. orientation 1)
    [[1, 0], [1, 1], [1, 2], [1, 3]],
];

// O-piece (square), orientation 0:
// X X
// X X
//
// The square looks identical in every orientation, so all 4 entries are the
// same cell list - "rotating" it is a visual no-op, but it still costs
// nothing extra since `rotate_cw`/`rotate_ccw` just index into this array.
const O_ORIENTATIONS: Orientations = [
    // orientation 0
    [[0, 0], [1, 0], [0, 1], [1, 1]],
    // orientation 1
    [[0, 0], [1, 0], [0, 1], [1, 1]],
    // orientation 2
    [[0, 0], [1, 0], [0, 1], [1, 1]],
    // orientation 3
    [[0, 0], [1, 0], [0, 1], [1, 1]],
];

// T-piece, orientation 0 - a 3-wide bar with a bump centered underneath:
// X X X
// . X .
const T_ORIENTATIONS: Orientations = [
    [[0, 1], [1, 1], [2, 1], [1, 2]],
    [[1, 0], [1, 1], [1, 2], [2, 1]],
    [[0, 1], [1, 1], [2, 1], [1, 0]],
    [[0, 1], [1, 0], [1, 1], [1, 2]],
];

// S-piece, orientation 0 - the top pair offset one column right of the
// bottom pair:
// . X X
// X X .
const S_ORIENTATIONS: Orientations = [
    [[1, 1], [2, 1], [0, 2], [1, 2]],
    [[1, 0], [1, 1], [2, 1], [2, 2]],
    [[1, 0], [2, 0], [0, 1], [1, 1]],
    [[0, 0], [0, 1], [1, 1], [1, 2]],
];

// Z-piece, orientation 0 - the mirror image of the S-piece (top pair offset
// one column left of the bottom pair):
// X X .
// . X X
const Z_ORIENTATIONS: Orientations = [
    [[0, 1], [1, 1], [1, 2], [2, 2]],
    [[2, 0], [2, 1], [1, 1], [1, 2]],
    [[0, 0], [1, 0], [1, 1], [2, 1]],
    [[1, 0], [1, 1], [0, 1], [0, 2]],
];

// J-piece, orientation 0 - a corner with the foot pointing left:
// X . .
// X X X
const J_ORIENTATIONS: Orientations = [
    [[0, 1], [0, 2], [1, 2], [2, 2]],
    [[1, 0], [2, 0], [1, 1], [1, 2]],
    [[0, 1], [1, 1], [2, 1], [2, 2]],
    [[1, 0], [1, 1], [0, 2], [1, 2]],
];

// L-piece, orientation 0 - the mirror image of the J-piece (foot pointing
// right):
// . . X
// X X X
const L_ORIENTATIONS: Orientations = [
    [[2, 1], [0, 2], [1, 2], [2, 2]],
    [[1, 0], [1, 1], [1, 2], [2, 2]],
    [[0, 1], [1, 1], [2, 1], [0, 2]],
    [[0, 0], [1, 0], [1, 1], [1, 2]],
];

/// The full set of 7 tetrominoes. This is a `const` array (no heap
/// allocation, no runtime initialization) since every orientation is a fixed,
/// known-at-compile-time list of cells.
///
/// The array's index *is* the `shape_idx` used everywhere else (on
/// `Tetromino` and as the values shuffled by [`generate_bag`]), so the order
/// here matters: index 0 is always the I-piece, 1 is always the O-piece, etc.
pub const SHAPES: [TetrominoShape; 7] = [
    TetrominoShape {
        orientations: I_ORIENTATIONS,
        color_index: ShapeColor::Cyan as u8,
        name: 'I',
    },
    TetrominoShape {
        orientations: O_ORIENTATIONS,
        color_index: ShapeColor::Yellow as u8,
        name: 'O',
    },
    TetrominoShape {
        orientations: T_ORIENTATIONS,
        color_index: ShapeColor::Magenta as u8,
        name: 'T',
    },
    TetrominoShape {
        orientations: S_ORIENTATIONS,
        color_index: ShapeColor::Green as u8,
        name: 'S',
    },
    TetrominoShape {
        orientations: Z_ORIENTATIONS,
        color_index: ShapeColor::Red as u8,
        name: 'Z',
    },
    TetrominoShape {
        orientations: J_ORIENTATIONS,
        color_index: ShapeColor::Blue as u8,
        name: 'J',
    },
    TetrominoShape {
        orientations: L_ORIENTATIONS,
        color_index: ShapeColor::DarkYellow as u8,
        name: 'L',
    },
];

/// "7-bag" randomizer: shuffles all 7 shape indices (0..=6, matching
/// [`SHAPES`]) into one random permutation. Unlike picking a piece uniformly
/// at random each time, drawing pieces from successive shuffled bags
/// guarantees every shape appears exactly once per 7 pieces, so the player
/// can never get an unlucky long drought (or flood) of the same piece - this
/// is the standard modern-Tetris randomizer. `Game` keeps a `bag` and a
/// pre-shuffled `next_bag` so it can always hand out a piece without
/// blocking on a fresh shuffle (see `game.rs`).
pub fn generate_bag() -> Vec<usize> {
    let mut bag = [0, 1, 2, 3, 4, 5, 6];
    let mut rng = rng();
    bag.shuffle(&mut rng);
    bag.to_vec()
}

/// Convert a shape's `color_index` (set on [`TetrominoShape`] from
/// [`ShapeColor`]) into the matching [`crossterm`] color used for rendering.
/// This indirection is what lets [`SHAPES`] stay a plain-data `const`: colors
/// are only resolved to a real `Color` value lazily, at render time.
use crossterm::style::Color;
pub fn color_from_index(index: u8) -> Color {
    match index {
        0 => Color::Cyan,
        1 => Color::Yellow,
        2 => Color::Magenta,
        3 => Color::Green,
        4 => Color::Red,
        5 => Color::Blue,
        6 => Color::DarkYellow,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_bag_is_a_permutation_of_zero_to_six() {
        let mut bag = generate_bag();
        assert_eq!(bag.len(), 7);
        bag.sort();
        assert_eq!(bag, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn color_from_index_covers_all_shapes_and_default() {
        assert_eq!(color_from_index(0), Color::Cyan);
        assert_eq!(color_from_index(1), Color::Yellow);
        assert_eq!(color_from_index(2), Color::Magenta);
        assert_eq!(color_from_index(3), Color::Green);
        assert_eq!(color_from_index(4), Color::Red);
        assert_eq!(color_from_index(5), Color::Blue);
        assert_eq!(color_from_index(6), Color::DarkYellow);
        // Out-of-range index falls back to white.
        assert_eq!(color_from_index(99), Color::White);
    }

    #[test]
    fn shapes_table_has_seven_named_tetrominoes() {
        assert_eq!(SHAPES.len(), 7);
        let names: Vec<char> = SHAPES.iter().map(|s| s.name).collect();
        assert_eq!(names, vec!['I', 'O', 'T', 'S', 'Z', 'J', 'L']);
    }

    #[test]
    fn every_orientation_has_four_cells() {
        for shape in SHAPES.iter() {
            for orientation in shape.orientations.iter() {
                assert_eq!(orientation.len(), 4);
            }
        }
    }
}
