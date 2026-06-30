//! Integration tests exercising the public `tetris_cli` library API end to end.

use crossterm::event::{KeyCode, KeyModifiers};
use tetris_cli::game::{Game, BOARD_HEIGHT, BOARD_WIDTH};
use tetris_cli::input::{action_for_key, apply_action, Action};

#[test]
fn test_new_game() {
    let game = Game::new();
    assert_eq!(game.board.len(), BOARD_HEIGHT);
    assert_eq!(game.board[0].len(), BOARD_WIDTH);
    assert!(!game.game_over);
    assert_eq!(game.score, 0);
    assert_eq!(game.level, 0);
    assert_eq!(game.lines_cleared, 0);
    assert!(game.can_hold);
    assert!(game.hold_piece.is_none());
    // The look-ahead holds two pieces (filled to three, one consumed to spawn).
    assert_eq!(game.next_queue.len(), 2);
}

#[test]
fn test_soft_drop() {
    let mut game = Game::new();
    let old_y = game.current_piece.y;
    game.try_move(0, 1);
    assert_eq!(game.current_piece.y, old_y + 1);
}

#[test]
fn test_hard_drop_locks_four_cells() {
    let mut game = Game::new();
    game.hard_drop();
    let filled = game.board.iter().flatten().filter(|c| c.is_some()).count();
    assert_eq!(filled, 4);
}

#[test]
fn test_clear_full_bottom_line() {
    let mut game = Game::new();
    // Fill the bottom row completely via the public board field.
    for x in 0..BOARD_WIDTH {
        game.board[BOARD_HEIGHT - 1][x] = Some(crossterm::style::Color::Cyan);
    }
    game.clear_lines();
    // Row cleared, score awarded, line counted.
    assert!(game.board[BOARD_HEIGHT - 1].iter().all(|c| c.is_none()));
    assert_eq!(game.lines_cleared, 1);
    assert_eq!(game.score, 40); // single line at level 0
}

#[test]
fn test_hold_then_swap() {
    let mut game = Game::new();
    let first = game.current_piece.shape_idx;
    game.hold_current_piece(); // None branch: stores current, spawns next
    assert_eq!(game.hold_piece.as_ref().unwrap().shape_idx, first);

    // Swap back: the Some branch returns the held piece and locks holding out.
    let current_before = game.current_piece.shape_idx;
    game.hold_current_piece(); // Some branch: swaps
    assert_eq!(game.current_piece.shape_idx, first);
    assert_eq!(game.hold_piece.as_ref().unwrap().shape_idx, current_before);
    assert!(!game.can_hold);
}

#[test]
fn test_game_over_on_blocked_spawn() {
    let mut game = Game::new();
    // Fill the spawn region so the next piece cannot be placed.
    for row in game.board.iter_mut().take(4) {
        for cell in row.iter_mut() {
            *cell = Some(crossterm::style::Color::Red);
        }
    }
    game.spawn_next_piece();
    assert!(game.game_over);
}

#[test]
fn test_input_flow_quit() {
    let mut game = Game::new();
    let action = action_for_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    assert_eq!(action, Action::Quit);
    assert!(apply_action(&mut game, action));
}

#[test]
fn test_input_flow_movement() {
    let mut game = Game::new();
    let x = game.current_piece.x;
    let action = action_for_key(KeyCode::Right, KeyModifiers::NONE).unwrap();
    assert!(!apply_action(&mut game, action));
    assert_eq!(game.current_piece.x, x + 1);
}
