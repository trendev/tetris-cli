use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{PrintStyledContent, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use std::io::{stdout, Write};
use std::time::Duration;

mod game;
mod shapes; // bring in shapes.rs // bring in game.rs

use game::{Game, BOARD_HEIGHT, BOARD_WIDTH};

fn main() -> crossterm::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(Hide)?;
    stdout.execute(Clear(ClearType::All))?;

    let mut game = Game::new();

    // Main loop
    'game_loop: loop {
        // 1) Input
        if poll(Duration::from_millis(1000))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = read()?
            {
                match code {
                    KeyCode::Left => {
                        game.try_move(-1, 0);
                    }
                    KeyCode::Right => {
                        game.try_move(1, 0);
                    }
                    KeyCode::Down => {
                        // Soft drop
                        if !game.try_move(0, 1) {
                            game.lock_piece();
                            game.clear_lines();
                            game.spawn_next_piece();
                        }
                    }
                    KeyCode::Up => {
                        // rotate cw
                        game.try_rotate_cw();
                    }
                    KeyCode::Char('z') => {
                        // rotate ccw
                        game.try_rotate_ccw();
                    }
                    KeyCode::Char('c') => {
                        // hold piece
                        game.hold_current_piece();
                    }
                    KeyCode::Char(' ') => {
                        // Hard drop
                        while game.try_move(0, 1) {}
                        game.lock_piece();
                        game.clear_lines();
                        game.spawn_next_piece();
                    }
                    KeyCode::Esc => {
                        break 'game_loop;
                    }
                    _ => {}
                }
                // Also allow Ctrl+C to quit
                if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                    break 'game_loop;
                }
            }
        }

        // 2) Update game (gravity)
        if !game.game_over {
            game.update();
        }

        // 3) Render
        stdout.queue(MoveTo(0, 0))?.queue(Clear(ClearType::All))?;

        // Draw board
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                if let Some(color) = game.board[y][x] {
                    let block = "█".with(color);
                    stdout.queue(PrintStyledContent(block))?;
                } else {
                    stdout.queue(PrintStyledContent("·".grey()))?;
                }
            }
            stdout.queue(PrintStyledContent("\r\n".stylize()))?;
        }

        // Draw current piece
        if !game.game_over {
            for &(px, py) in game.current_piece.positions().iter() {
                if px >= 0 && px < BOARD_WIDTH as i32 && py >= 0 && py < BOARD_HEIGHT as i32 {
                    stdout
                        .queue(MoveTo(px as u16, py as u16))?
                        .queue(PrintStyledContent("█".with(game.current_piece.color())))?;
                }
            }
        }

        // Draw hold piece
        stdout
            .queue(MoveTo(0, BOARD_HEIGHT as u16 + 1))?
            .queue(PrintStyledContent("Hold: ".to_string().stylize()))?;
        if let Some(hp) = &game.hold_piece {
            stdout.queue(PrintStyledContent(format!("{}", hp.name()).stylize()))?;
        } else {
            stdout.queue(PrintStyledContent("None".to_string().stylize()))?;
        }

        // Draw next pieces
        stdout
            .queue(MoveTo(0, BOARD_HEIGHT as u16 + 2))?
            .queue(PrintStyledContent("Next: ".to_string().stylize()))?;
        for (_i, &shape_idx) in game.next_queue.iter().take(3).enumerate() {
            let shape_char = shapes::SHAPES[shape_idx].name;
            stdout.queue(PrintStyledContent(format!("{} ", shape_char).stylize()))?;
        }

        // Stats
        stdout
            .queue(MoveTo(0, BOARD_HEIGHT as u16 + 4))?
            .queue(PrintStyledContent(
                format!("Score: {}", game.score).stylize(),
            ))?;
        stdout
            .queue(MoveTo(0, BOARD_HEIGHT as u16 + 5))?
            .queue(PrintStyledContent(
                format!("Level: {}", game.level).stylize(),
            ))?;
        stdout
            .queue(MoveTo(0, BOARD_HEIGHT as u16 + 6))?
            .queue(PrintStyledContent(
                format!("Lines: {}", game.lines_cleared).stylize(),
            ))?;

        if game.game_over {
            stdout
                .queue(MoveTo(0, BOARD_HEIGHT as u16 + 8))?
                .queue(PrintStyledContent("GAME OVER".red()))?;
        }

        stdout.flush()?;

        // If game over, pause briefly and exit
        if game.game_over {
            std::thread::sleep(Duration::from_secs(3));
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    stdout.execute(Show)?;

    Ok(())
}
