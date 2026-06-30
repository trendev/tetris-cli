//! Binary entry point. This file deliberately contains no game logic: it only
//! owns the terminal (raw mode, cursor) and drives the input -> update -> render
//! loop, delegating everything else to the `tetris_cli` library. Keeping logic
//! out of here matters because this is the one file that needs a real TTY and
//! therefore cannot be unit-tested.
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
    // Enable raw mode so individual key presses (arrows, etc.) reach us
    // immediately instead of being line-buffered by the terminal, and hide the
    // cursor so it doesn't flicker over the board while we redraw each frame.
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(Hide)?;
    stdout.execute(Clear(ClearType::All))?;

    let mut game = Game::new();

    // Main loop: input -> update (gravity) -> render, once per iteration.
    loop {
        // 1) Input.
        // `poll` blocks for up to 1s waiting for a key event, which doubles as
        // our frame pacing when the player is idle: even with no key pressed,
        // the loop wakes at least once a second so gravity still gets a chance
        // to run via `game.update()` below. Any faster fall_interval is driven
        // by key presses (which return from `poll` immediately) keeping the
        // loop spinning.
        if poll(Duration::from_millis(1000))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = read()?
            {
                // `action_for_key` is a pure mapping (key -> Action) so it's
                // unit-tested without a terminal; only the actual key read
                // happens here. `apply_action` returns true to request quit.
                if let Some(action) = action_for_key(code, modifiers) {
                    if apply_action(&mut game, action) {
                        break;
                    }
                }
            }
        }

        // 2) Update game (gravity). Skipped once the game is over so the
        // locked board stays frozen for the final render.
        if !game.game_over {
            game.update();
        }

        // 3) Render the current frame.
        render(&mut stdout, &game)?;

        // Once game over, leave the "GAME OVER" banner on screen briefly
        // before tearing down the terminal.
        if game.game_over {
            std::thread::sleep(Duration::from_secs(3));
            break;
        }
    }

    // Restore the terminal to its normal state no matter how we exited the loop.
    disable_raw_mode()?;
    stdout.execute(Show)?;

    Ok(())
}
