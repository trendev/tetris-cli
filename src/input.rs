use crossterm::event::{KeyCode, KeyModifiers};

use crate::game::Game;

/// A high-level game action decoded from a key press. Keeping this separate from
/// crossterm event handling makes the input mapping pure and unit-testable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCw,
    RotateCcw,
    Hold,
    Quit,
}

/// Map a key press to an [`Action`], or `None` if the key is unbound.
pub fn action_for_key(code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    // Ctrl+C quits regardless of which character key it is paired with.
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    match code {
        KeyCode::Left => Some(Action::MoveLeft),
        KeyCode::Right => Some(Action::MoveRight),
        KeyCode::Down => Some(Action::SoftDrop),
        KeyCode::Up => Some(Action::RotateCw),
        KeyCode::Char('z') => Some(Action::RotateCcw),
        KeyCode::Char('c') => Some(Action::Hold),
        KeyCode::Char(' ') => Some(Action::HardDrop),
        KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}

/// Apply an [`Action`] to the game. Returns `true` if the game should quit.
pub fn apply_action(game: &mut Game, action: Action) -> bool {
    match action {
        Action::MoveLeft => {
            game.try_move(-1, 0);
        }
        Action::MoveRight => {
            game.try_move(1, 0);
        }
        Action::SoftDrop => game.soft_drop(),
        Action::HardDrop => game.hard_drop(),
        Action::RotateCw => game.try_rotate_cw(),
        Action::RotateCcw => game.try_rotate_ccw(),
        Action::Hold => game.hold_current_piece(),
        Action::Quit => return true,
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(code: KeyCode) -> Option<Action> {
        action_for_key(code, KeyModifiers::NONE)
    }

    #[test]
    fn maps_each_bound_key() {
        assert_eq!(act(KeyCode::Left), Some(Action::MoveLeft));
        assert_eq!(act(KeyCode::Right), Some(Action::MoveRight));
        assert_eq!(act(KeyCode::Down), Some(Action::SoftDrop));
        assert_eq!(act(KeyCode::Up), Some(Action::RotateCw));
        assert_eq!(act(KeyCode::Char('z')), Some(Action::RotateCcw));
        assert_eq!(act(KeyCode::Char('c')), Some(Action::Hold));
        assert_eq!(act(KeyCode::Char(' ')), Some(Action::HardDrop));
        assert_eq!(act(KeyCode::Esc), Some(Action::Quit));
    }

    #[test]
    fn unbound_key_is_none() {
        assert_eq!(act(KeyCode::Char('q')), None);
        assert_eq!(act(KeyCode::Tab), None);
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(
            action_for_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Action::Quit)
        );
    }

    #[test]
    fn apply_quit_returns_true() {
        let mut game = Game::new();
        assert!(apply_action(&mut game, Action::Quit));
    }

    #[test]
    fn apply_move_right_shifts_piece() {
        let mut game = Game::new();
        let x = game.current_piece.x;
        let quit = apply_action(&mut game, Action::MoveRight);
        assert!(!quit);
        assert_eq!(game.current_piece.x, x + 1);
    }

    #[test]
    fn apply_move_left_shifts_piece() {
        let mut game = Game::new();
        let x = game.current_piece.x;
        apply_action(&mut game, Action::MoveLeft);
        assert_eq!(game.current_piece.x, x - 1);
    }

    #[test]
    fn apply_rotations_change_orientation() {
        let mut game = Game::new();
        apply_action(&mut game, Action::RotateCw);
        apply_action(&mut game, Action::RotateCcw);
        // Round trip leaves orientation back at the start.
        assert_eq!(game.current_piece.orientation, 0);
    }

    #[test]
    fn apply_hold_stores_piece() {
        let mut game = Game::new();
        assert!(game.hold_piece.is_none());
        apply_action(&mut game, Action::Hold);
        assert!(game.hold_piece.is_some());
    }

    #[test]
    fn apply_hard_drop_locks_and_advances() {
        let mut game = Game::new();
        apply_action(&mut game, Action::HardDrop);
        // The board now has the locked piece's four cells.
        let filled: usize = game
            .board
            .iter()
            .flatten()
            .filter(|cell| cell.is_some())
            .count();
        assert_eq!(filled, 4);
    }

    #[test]
    fn apply_soft_drop_moves_down() {
        let mut game = Game::new();
        let y = game.current_piece.y;
        apply_action(&mut game, Action::SoftDrop);
        assert_eq!(game.current_piece.y, y + 1);
    }
}
