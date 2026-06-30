# Tetris CLI (Rust)

A simple command-line Tetris clone with classic features:

- **7-Bag Randomizer** (modern Tetris style)  
- **Piece Hold** (`C` key)  
- **Next Piece Preview** (3 pieces)  
- **Soft Drop** (Down arrow), **Hard Drop** (Space)  
- **Scoring & Levels** (increasing speed)  

## Getting Started

### Prerequisites

- A [Rust toolchain](https://www.rust-lang.org/tools/install) (stable, 2021 edition) — `cargo` and `rustc`.
- An interactive terminal (TTY). The game enables raw mode, so it cannot run headless or in CI.

### Build

```bash
cargo build            # debug build
cargo build --release  # optimized build (binary in target/release/tetris-cli)
```

### Run

```bash
cargo run              # build and play
./target/release/tetris-cli   # run a release binary directly
```

> **Note:** `cargo run` needs a real interactive terminal because it switches the
> terminal into raw mode. It won't work when piped, redirected, or run in CI.

### Test

```bash
cargo test                                    # run all unit + integration tests
cargo test test_clear_tetris_and_non_adjacent  # run a single test by name
```

All game logic lives in the `tetris_cli` library, so tests run fully headless —
no terminal required.

### Lint, Format & Coverage

These mirror the CI gate (`.github/workflows/main.yml`); run them before pushing:

```bash
cargo fmt --all --check                                     # verify formatting
cargo clippy --all-targets --all-features -- -D warnings    # lint (warnings fail)
cargo llvm-cov --summary-only                               # coverage (cargo install cargo-llvm-cov)
```

## How to Play

The controls are also listed live in-game, in a panel below the score.

- **Left / Right Arrows**: Move the falling piece left or right  
- **Down Arrow**: Soft drop (speeds up piece descent)  
- **Up Arrow**: Rotate the piece clockwise  
- **`Z`**: Rotate the piece counterclockwise  
- **`Space`**: Hard drop (instantly drops the piece to the bottom)  
- **`C`**: Hold the current piece (swap with the previously held piece)  
- **`Esc`** or **Ctrl + C**: Quit the game  

### Scoring

- **Single Line Clear**: 40 points × (level + 1)  
- **Double**: 100 points × (level + 1)  
- **Triple**: 300 points × (level + 1)  
- **Tetris (4 lines)**: 1200 points × (level + 1)  

### Levels & Speed

- **Level** increases every 10 lines cleared.  
- As the level goes up, pieces drop faster (gravity interval decreases).  

Enjoy the classic Tetris experience right in your terminal!
