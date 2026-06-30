//! Library crate for the Tetris CLI game.
//!
//! All game logic lives here so it can be unit-tested without a real terminal.
//! `src/main.rs` is a thin binary that only handles terminal setup/teardown
//! and the event loop; it calls into the modules below.
//!
//! - [`game`]   - the `Game` state machine: board, active piece, scoring, lock cycle.
//! - [`shapes`] - the 7 tetromino shapes and their precomputed rotations.
//! - [`input`]  - maps key presses to game actions (pure, no terminal dependency).
//! - [`render`] - draws the game state to any `Write`r (terminal or in-memory buffer).
pub mod game;
pub mod input;
pub mod render;
pub mod shapes;
