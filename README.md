# Grail Chess Engine

Grail is a UCI-compliant chess engine written in Rust, featuring both Neural Network (NNUE) and Hand-Crafted (HCE) evaluation. It implements modern search techniques including Principal Variation Search, advanced pruning, and sophisticated move ordering.

## Download & Installation

Grail is a command-line engine and requires a chess GUI to play.

1.  **Download**: Get the latest executable from the [Releases](../../releases) page for your operating system (Windows, macOS, or Linux).
2.  **Install GUI**: Download a UCI-compatible chess GUI (such as Arena or BanksiaGUI).
3.  **Setup**:
    - Open the GUI.
    - Add a new engine.
    - Select the `grail` executable.
    - Start a game!

### UCI Options

Configure Grail through the GUI settings:

- **Hash**: Size of the transposition table in MB (default: 1024).
- **Use NNUE**: Toggle between NNUE and HCE evaluation (default: true).

## Play Against Grail Online

You can challenge the latest version of Grail on Lichess at [lichess.org/@/grail-bot](https://lichess.org/@/grail-bot), running on a 1 vCPU Northflank instance with 1024 MB hash.

## For Developers

### Building from Source

**Prerequisites:** Rust nightly toolchain (Grail uses `portable_simd`).

**Quick Start:**

```bash
git clone https://github.com/jorgenhanssen/grail.git
cd grail
rustup override set nightly
make grail
```

This builds the release binary at `target/release/grail`.

### Build Targets

The project includes a `Makefile` for convenience:

- **`make grail`** (default): Builds the release binary.
- **`make grail-tuning`**: Builds with exposed parameters for SPSA tuning.
- **`make generate`**: Builds the data generation tool for NNUE training.
- **`make train`**: Builds the NNUE trainer (auto-detects CUDA/Metal).
- **`make clean`**: Cleans the build directory.

### Features & Architecture

#### Search

- **Framework**: Negamax search with Alpha-Beta pruning and Principal Variation Search (PVS).
- **Iterative Deepening**: Progressively deeper searches to improve move ordering.
- **Transposition Table**: Stores search results to avoid re-searching positions.
- **Aspiration Windows**: Narrows the search window around the previous score for efficiency.
- **Internal Iterative Deepening (IID)**: Performs a shallower search to get a best move when the Transposition Table misses.

#### Pruning Techniques

- **Reverse Futility Pruning (RFP)**: Prunes nodes where the static evaluation is significantly above beta.
- **Null Move Pruning (NMP)**: Assumes the opponent moves twice; if the position is still too good, the branch is pruned.
- **Late Move Pruning (LMP)**: Prunes quiet moves late in the move list at low depths.
- **Futility Pruning**: Prunes moves that are unlikely to raise alpha.
- **Razoring**: Prunes nodes at low depth if the static evaluation is far below alpha.
- **SEE Pruning**: Prunes moves with negative Static Exchange Evaluation scores at low depths.
- **Mate Distance Pruning**: Prunes paths that cannot lead to a faster mate than one already found.
- **Delta Pruning**: Used in Quiescence Search to prune captures that cannot improve the position.

#### Reductions

- **Late Move Reductions (LMR)**: Reduces the search depth for quiet moves later in the ordering, assuming better moves were found earlier.

#### Move Ordering

Efficient move ordering is critical for alpha-beta performance. Grail uses:

1.  **Transposition Table Move**: The best move from a previous search.
2.  **Killer Heuristic**: Quiet moves that caused a beta cutoff at the same ply in sibling nodes.
3.  **History Heuristics**:
    - **History Heuristic**: Prioritizes quiet moves that have frequently been good elsewhere in the tree.
    - **Capture History**: Orders captures based on historical success.
    - **Continuation History**: Heuristics based on previous moves (counter moves/follow-up moves).
4.  **Winning Captures**: Ordered by MVV-LVA (Most Valuable Victim - Least Valuable Attacker) logic.

#### Evaluation

- **NNUE**: The default evaluation. Uses a small, efficiently updatable neural network trained on engine self-play data.
- **HCE**: A fallback hand-crafted evaluation function considering material, piece-square tables, pawn structure, king safety, and mobility.

### NNUE Data Generation & Training

Grail includes tools to generate training data and train its own NNUE networks.

#### Data Generation

Generate self-play games to create a dataset:

```bash
make generate
./target/release/generate --book books/your_opening_book.epd
```

**Arguments:**

- `--book`: Path to an opening book in EPD format (required).
- `--depth`: Search depth for each move (default: 10).
- `--nnue`: Use NNUE for generation (default: false, uses HCE).

Generated data is saved to `nnue/data/YYYY-MM-DD-HH:MM.csv`.

#### Training

Train a new network using the generated data:

```bash
make train  # Auto-detects GPU support (CUDA/Metal)
./target/release/train
```

The trainer loads all CSV files from `nnue/data/` and saves the best model to `nnue/model.safetensors`.

**Arguments:**

- `--batch-size`: Batch size for training (default: 8192).
- `--learning-rate`: Initial learning rate (default: 0.001).
- `--epochs`: Number of epochs to train (default: 100).
- `--workers`: Number of worker threads for data loading (default: 4).
- `--val-ratio`: Fraction of data to use for validation (default: 0.1).
- `--test-ratio`: Fraction of data to use for testing (default: 0.01).
- `--lr-decay`: Learning rate decay factor (default: 0.95).
- `--patience`: Epochs to wait for improvement before stopping (default: 2).
