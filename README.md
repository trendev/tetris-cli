# Tetris CLI (Rust)

A simple command-line Tetris clone with classic features:

- **7-Bag Randomizer** (modern Tetris style)  
- **Piece Hold** (`C` key)  
- **Next Piece Preview** (3 pieces)  
- **Soft Drop** (Down arrow), **Hard Drop** (Space)  
- **Scoring & Levels** (increasing speed)  

## How to Play

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
