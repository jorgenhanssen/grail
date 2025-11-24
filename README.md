# Grail Chess Engine

Grail is a UCI-compliant hobby chess engine written in Rust, featuring both Neural Network (NNUE) and Hand-Crafted (HCE) evaluation. It implements modern search techniques including Principal Variation Search, advanced pruning, and sophisticated move ordering.

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
