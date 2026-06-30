use std::io::{Result, Write};

use crossterm::{
    cursor::MoveTo,
    style::{PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};

use crate::game::{Game, BOARD_HEIGHT, BOARD_WIDTH};
use crate::shapes::SHAPES;

/// A filled board cell. Two full-block chars wide so cells render square despite the
/// terminal's ~1:2 (width:height) character aspect ratio.
const BLOCK: &str = "\u{2588}\u{2588}";
/// An empty board cell: a faint dot plus a space, keeping a light playfield grid while
/// occupying the same two columns as a [`BLOCK`].
const EMPTY: &str = "\u{00b7} ";

/// Key bindings shown in-game so players know how to play without consulting the README.
const CONTROLS: [&str; 8] = [
    "Controls:",
    "  \u{2190} \u{2192}  Move",
    "  \u{2191}    Rotate CW",
    "  Z    Rotate CCW",
    "  \u{2193}    Soft drop",
    "  Spc  Hard drop",
    "  C    Hold",
    "  Esc  Quit",
];

/// Draw the full game frame (board, active piece, hold, next queue and stats) to `out`.
///
/// Writing to a generic [`Write`] keeps rendering decoupled from the terminal so it can be
/// exercised against an in-memory buffer in tests.
pub fn render<W: Write>(out: &mut W, game: &Game) -> Result<()> {
    out.queue(MoveTo(0, 0))?.queue(Clear(ClearType::All))?;

    // Draw the board. Each cell is two columns wide so minos render square.
    for row in game.board.iter() {
        for cell in row.iter() {
            match cell {
                Some(color) => {
                    out.queue(PrintStyledContent(BLOCK.with(*color)))?;
                }
                None => {
                    out.queue(PrintStyledContent(EMPTY.dark_grey()))?;
                }
            }
        }
        out.queue(PrintStyledContent("\r\n".stylize()))?;
    }

    // Draw the active piece on top of the board (cell x maps to screen column 2*x).
    if !game.game_over {
        for &(px, py) in game.current_piece.positions().iter() {
            if px >= 0 && px < BOARD_WIDTH as i32 && py >= 0 && py < BOARD_HEIGHT as i32 {
                out.queue(MoveTo((px * 2) as u16, py as u16))?
                    .queue(PrintStyledContent(BLOCK.with(game.current_piece.color())))?;
            }
        }
    }

    // Hold slot.
    out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 1))?
        .queue(PrintStyledContent("Hold: ".stylize()))?;
    match &game.hold_piece {
        Some(hp) => {
            out.queue(PrintStyledContent(hp.name().to_string().stylize()))?;
        }
        None => {
            out.queue(PrintStyledContent("None".stylize()))?;
        }
    }

    // Next queue.
    out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 2))?
        .queue(PrintStyledContent("Next: ".stylize()))?;
    for &shape_idx in game.next_queue.iter().take(3) {
        let shape_char = SHAPES[shape_idx].name;
        out.queue(PrintStyledContent(format!("{shape_char} ").stylize()))?;
    }

    // Stats.
    out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 4))?
        .queue(PrintStyledContent(
            format!("Score: {}", game.score).stylize(),
        ))?;
    out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 5))?
        .queue(PrintStyledContent(
            format!("Level: {}", game.level).stylize(),
        ))?;
    out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 6))?
        .queue(PrintStyledContent(
            format!("Lines: {}", game.lines_cleared).stylize(),
        ))?;

    // Controls legend (dimmed so it doesn't compete with the board).
    for (i, line) in CONTROLS.iter().enumerate() {
        out.queue(MoveTo(0, BOARD_HEIGHT as u16 + 8 + i as u16))?
            .queue(PrintStyledContent(line.dark_grey()))?;
    }

    if game.game_over {
        // Row derived from `CONTROLS.len()` rather than a hardcoded number,
        // so the banner automatically drops one row lower if a line is ever
        // added to the legend above - don't replace this with a literal.
        out.queue(MoveTo(
            0,
            BOARD_HEIGHT as u16 + 8 + CONTROLS.len() as u16 + 1,
        ))?
        .queue(PrintStyledContent("GAME OVER".red()))?;
    }

    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(game: &Game) -> String {
        let mut buf = Vec::new();
        render(&mut buf, game).expect("render should succeed to an in-memory buffer");
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn renders_labels_and_stats() {
        let game = Game::new();
        let out = render_to_string(&game);
        assert!(out.contains("Hold:"));
        assert!(out.contains("None")); // no hold piece on a fresh game
        assert!(out.contains("Next:"));
        assert!(out.contains("Score: 0"));
        assert!(out.contains("Level: 0"));
        assert!(out.contains("Lines: 0"));
        assert!(out.contains("Controls:"));
        assert!(out.contains("Hard drop"));
        assert!(!out.contains("GAME OVER"));
        // A fresh game draws its active piece as a square block, over a dotted grid.
        assert!(out.contains(BLOCK));
        assert!(out.contains(EMPTY));
    }

    #[test]
    fn renders_square_blocks_after_lock() {
        let mut game = Game::new();
        game.hard_drop();
        let out = render_to_string(&game);
        // Locked cells render as solid square blocks.
        assert!(out.contains(BLOCK));
    }

    #[test]
    fn renders_game_over_banner() {
        let mut game = Game::new();
        game.game_over = true;
        let out = render_to_string(&game);
        assert!(out.contains("GAME OVER"));
    }

    #[test]
    fn renders_held_piece_name() {
        let mut game = Game::new();
        game.hold_current_piece();
        let out = render_to_string(&game);
        let name = game.hold_piece.as_ref().unwrap().name();
        assert!(out.contains(&name.to_string()));
    }

    #[test]
    fn renders_locked_cells() {
        let mut game = Game::new();
        game.hard_drop();
        // Rendering a board with locked cells must succeed and still show stats.
        let out = render_to_string(&game);
        assert!(out.contains("Score:"));
    }
}
