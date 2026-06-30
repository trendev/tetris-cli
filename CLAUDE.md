# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

A command-line Tetris clone in Rust (7-bag randomizer, hold, next-piece preview, soft/hard drop, scoring & levels). See `README.md` for gameplay controls and the scoring table.

## Commands

```bash
cargo build                  # build
cargo run                    # play — needs an interactive TTY (enables raw mode); won't run headless/CI
cargo test                   # all unit + integration tests
cargo test test_clear_tetris_and_non_adjacent   # run a single test by name
cargo fmt --all              # format (--check to verify only)
cargo clippy --all-targets --all-features -- -D warnings   # lint; -D warnings is the CI gate
cargo llvm-cov --summary-only                              # coverage report (cargo install cargo-llvm-cov)
```

CI (`.github/workflows/main.yml`) gates on `fmt --check`, `clippy -D warnings`, tests, and coverage. Run those four locally before pushing or the build fails.

## Architecture

**Library/binary split is the core design choice.** All game logic lives in the `tetris_cli` library (`src/lib.rs` exposes `game`, `shapes`, `input`, `render`). `src/main.rs` is a thin binary that only owns terminal setup/teardown (raw mode, cursor) and the event loop — it consumes the library. **Keep logic out of `main.rs`**: it's the one file with no test coverage by design (it needs a real terminal), so anything testable belongs in a library module.

**Two patterns make the code testable** — preserve them when extending:
- `render::render<W: Write>(out, game)` writes to a generic writer, so tests render into a `Vec<u8>` and assert on the bytes instead of needing a terminal.
- Input is split into a *pure* `input::action_for_key(code, modifiers) -> Option<Action>` and an `apply_action(game, action)`. The crossterm event read stays in `main.rs`; the mapping and effects are unit-tested.

`render` also draws a static controls legend from the `CONTROLS` const; the `GAME OVER` banner's row is derived from `CONTROLS.len()`, so adding a legend line shifts the banner automatically — don't hardcode that offset. Keep `CONTROLS` in sync with `action_for_key` when you change a key binding.

**`src/game.rs` is the state machine.** `Game` holds the board + active `Tetromino` + bags + score/level. Lock cycle: `update()` (gravity, fires when `fall_interval` elapses) → `soft_drop()` → on collision `lock_and_advance()` = `lock_piece()` → `clear_lines()` → `spawn_next_piece()`. Both `main.rs` key handling and gravity route through these `Game` methods — don't re-inline the lock sequence.

**`src/shapes.rs`** defines `SHAPES`: a compile-time `const` array of all 7 tetrominoes. Each shape's 4 orientations are **hardcoded cell lists**, not derived by rotation math — rotation just indexes `orientation` 0..3. `generate_bag()` is the 7-bag randomizer (`rand`).

### Conventions & gotchas

- Coordinates are `[x, y]` offsets; the board is row-major: `board[y][x]`. `Tetromino::positions()` returns absolute `(x, y)` tuples.
- `next_queue` holds **2** pieces after `new()`/`spawn_next_piece()` (filled to 3, one consumed to spawn) — not 3.
- The **first hold** (the `None` branch of `hold_current_piece`) calls `spawn_next_piece`, which resets `can_hold = true` — so the first hold does not lock out further holding. The swap (`Some`) branch leaves `can_hold = false`. Match this when asserting hold behavior.
- Line scoring uses the level *before* a level-up: clearing the 10th line scores at the old level, then bumps the level.
- Wallkick (`do_wallkick`) is a naive fixed offset list, not SRS.
