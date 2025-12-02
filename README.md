# Grail Chess Engine

[![CI](https://github.com/jorgenhanssen/grail/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/jorgenhanssen/grail/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)

Grail is a hobby chess engine written in Rust, featuring both Neural Network (NNUE) and Hand-Crafted (HCE) evaluation. It implements modern search techniques including Principal Variation Search, advanced pruning, and sophisticated move ordering.

## Usage

Grail is a command-line engine designed for **Standard Chess**. It requires a UCI-compatible chess GUI (such as Arena, BanksiaGUI, or Cutechess) to play.

### Getting Started

1. **Download**: Grab the zip for your OS from the [Releases](../../releases) page and extract it.
2. **Install**: Open your chess GUI and add the right binary (see table below).
3. **Play**: Start a game or analysis session!

#### Which binary should I use?

Each release includes builds optimized for different CPU architectures:

| OS                  | Binary      | Supported CPUs                                        |
| ------------------- | ----------- | ----------------------------------------------------- |
| **Linux / Windows** | `x86-64-v4` | Intel Skylake-X/Ice Lake+ (2017+), AMD Zen 4+ (2022+) |
| **Linux / Windows** | `x86-64-v3` | Intel Haswell+ (2013+), AMD Zen 2+ (2019+)            |
| **macOS**           | `arm64`     | Apple Silicon (M1/M2/M3/M4)                           |

<!-- prettier-ignore -->
> [!TIP]
> **Not sure?** On Windows/Linux, try `x86-64-v4` first for best performance. If the engine crashes on startup, use `x86-64-v3` instead - it has wider compatibility.
>
> For technical details, see [x86-64 Microarchitecture Levels](https://en.wikipedia.org/wiki/X86-64#Microarchitecture_levels).

#### A note for macOS users

macOS blocks unsigned binaries by default. Apple wants me to pay $99/year to sign the binary so you can avoid typing this command. So instead, after downloading, just run:

```bash
xattr -d com.apple.quarantine ~/Downloads/grail-arm64
```

and you should be able to run it! 🍎

### Configuration

Once added to your GUI, you can configure the engine via the UCI options:

- **Hash**: Size of the transposition table in MB (Default: 1024).
- **NNUE**: Toggle between Neural Network (NNUE) and Hand-Crafted (HCE) evaluation (Default: true).

The engine supports standard time controls (increment, sudden death, moves to go) and analysis modes (fixed depth, infinite).

## Play Against Grail Online

You can challenge the latest version of Grail on [Lichess](https://lichess.org/@/grail-bot), running on a 1 vCPU Northflank instance with 1024 MB hash.

## For Developers

### Building from Source

**Prerequisites:** Rust nightly toolchain (Grail uses `portable_simd` and `generic_const_exprs`).

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

### Benchmarking

Runs depth-15 searches on standard perft positions:

```bash
cargo bench --bench search
```

For profiling with flamegraph:

```bash
CARGO_PROFILE_BENCH_DEBUG=true cargo flamegraph --bench search -- --bench
```

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

## Acknowledgements

- [Chess Programming Wiki](https://www.chessprogramming.org/) – An invaluable resource for chess programming concepts and techniques.
- Opening books in `/books` sourced from the computer chess community via [TalkChess](https://talkchess.com/viewtopic.php?t=50619).
