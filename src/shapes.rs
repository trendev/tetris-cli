use rand::seq::SliceRandom;
use rand::thread_rng;

/// We'll define a small color enum or store just a u8 ID.
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
const I_ORIENTATIONS: Orientations = [
    // orientation 0
    [[0,1],[1,1],[2,1],[3,1]],
    // orientation 1
    [[2,0],[2,1],[2,2],[2,3]],
    // orientation 2
    [[0,2],[1,2],[2,2],[3,2]],
    // orientation 3
    [[1,0],[1,1],[1,2],[1,3]],
];

const O_ORIENTATIONS: Orientations = [
    // orientation 0
    [[0,0],[1,0],[0,1],[1,1]],
    // orientation 1
    [[0,0],[1,0],[0,1],[1,1]],
    // orientation 2
    [[0,0],[1,0],[0,1],[1,1]],
    // orientation 3
    [[0,0],[1,0],[0,1],[1,1]],
];

const T_ORIENTATIONS: Orientations = [
    [[0,1],[1,1],[2,1],[1,2]],
    [[1,0],[1,1],[1,2],[2,1]],
    [[0,1],[1,1],[2,1],[1,0]],
    [[0,1],[1,0],[1,1],[1,2]],
];

const S_ORIENTATIONS: Orientations = [
    [[1,1],[2,1],[0,2],[1,2]],
    [[1,0],[1,1],[2,1],[2,2]],
    [[1,0],[2,0],[0,1],[1,1]],
    [[0,0],[0,1],[1,1],[1,2]],
];

const Z_ORIENTATIONS: Orientations = [
    [[0,1],[1,1],[1,2],[2,2]],
    [[2,0],[2,1],[1,1],[1,2]],
    [[0,0],[1,0],[1,1],[2,1]],
    [[1,0],[1,1],[0,1],[0,2]],
];

const J_ORIENTATIONS: Orientations = [
    [[0,1],[0,2],[1,2],[2,2]],
    [[1,0],[2,0],[1,1],[1,2]],
    [[0,1],[1,1],[2,1],[2,2]],
    [[1,0],[1,1],[0,2],[1,2]],
];

const L_ORIENTATIONS: Orientations = [
    [[2,1],[0,2],[1,2],[2,2]],
    [[1,0],[1,1],[1,2],[2,2]],
    [[0,1],[1,1],[2,1],[0,2]],
    [[0,0],[1,0],[1,1],[1,2]],
];

/// Now define an array of the 7 shapes. This is truly compile-time, no heap allocations.
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

/// 7-bag generator that just returns shape indices 0..6 in random order (runtime).
pub fn generate_bag() -> Vec<usize> {
    let mut bag = [0,1,2,3,4,5,6];
    let mut rng = thread_rng();
    bag.shuffle(&mut rng);
    bag.to_vec()
}

/// Convert a color index to an actual crossterm color (runtime).
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
