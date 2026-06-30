use std::io::{stdout, Result};
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};

use tetris_cli::game::Game;
use tetris_cli::input::{action_for_key, apply_action};
use tetris_cli::render::render;

fn main() -> Result<()> {
    // Setup terminal.
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(Hide)?;
    stdout.execute(Clear(ClearType::All))?;

    let mut game = Game::new();

    // Main loop.
    loop {
        // 1) Input
        if poll(Duration::from_millis(1000))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = read()?
            {
                if let Some(action) = action_for_key(code, modifiers) {
                    if apply_action(&mut game, action) {
                        break;
                    }
                }
            }
        }

        // 2) Update game (gravity)
        if !game.game_over {
            game.update();
        }

        // 3) Render
        render(&mut stdout, &game)?;

        // If game over, pause briefly and exit.
        if game.game_over {
            std::thread::sleep(Duration::from_secs(3));
            break;
        }
    }

    // Cleanup.
    disable_raw_mode()?;
    stdout.execute(Show)?;

    Ok(())
}
