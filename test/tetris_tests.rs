use tetris_cli::game::{Game, BOARD_HEIGHT, BOARD_WIDTH};

#[test]
fn test_new_game() {
    let game = Game::new();
    assert_eq!(game.board.len(), BOARD_HEIGHT);
    assert_eq!(game.board[0].len(), BOARD_WIDTH);
    assert!(!game.game_over);
    // Check initial score
    assert_eq!(game.score, 0);
}

#[test]
fn test_soft_drop() {
    let mut game = Game::new();
    // Try moving down repeatedly
    let old_y = game.current_piece.y;
    game.try_move(0, 1);
    assert_eq!(game.current_piece.y, old_y + 1);
}
