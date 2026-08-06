# Grail

[![CCRL 40/15](https://img.shields.io/badge/CCRL%2040%2F15-3389%20Elo-%23DAA520.svg)](https://computerchess.org.uk/ccrl/4040/cgi/compare_engines.cgi?family=Grail&print=Rating+list)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)

Grail is a hobby chess engine written in Rust. It began as an attempt to make a chess engine and has since become an elaborate system for turning my sanity and electricity bill into Elo. It uses modern search techniques and a fully self-taught NNUE trained on 99 million self-play games. The name refers to the Holy Grail, which may still be easier to find than perfect chess.

This repository hosts Grail's official releases and source code. The engine is developed entirely within this repository, which contains the self-play datagen, NNUE training pipeline, analysis tools, profiling, and build setup.

## Usage

Grail is a command-line UCI engine built for **Standard Chess**, so it requires a UCI-compatible chess GUI (such as Arena, BanksiaGUI, or Cutechess) to play.

1. **Download**: Grab the zip for your OS from the [Releases](../../releases) page and extract it.
2. **Install**: Open your chess GUI and add the right binary (see table below).
3. **Play**: Start a game or analysis session.

### Which binary should I use?

Each release includes builds for a few different CPU architectures:

| OS                      | Binary      | CPU compatibility                                     |
| ----------------------- | ----------- | ----------------------------------------------------- |
| **Linux / Windows**     | `x86-64-v4` | Intel Skylake-X/Ice Lake+ (2017+), AMD Zen 4+ (2022+) |
| **Linux / Windows**     | `x86-64-v3` | Intel Haswell+ (2013+), AMD Zen 2+ (2019+)            |
| **Linux ARM / Android** | `aarch64`   | Any 64-bit ARM CPU                                    |
| **macOS**               | `arm64`     | Apple Silicon (M1/M2/M3/M4)                           |

<!-- prettier-ignore -->
> [!TIP]
> **Not sure?** On Windows/Linux, try `x86-64-v4` first for best performance. If the engine crashes on startup, use `x86-64-v3` instead - it has wider compatibility.
>
> For technical details, see [x86-64 Microarchitecture Levels](https://en.wikipedia.org/wiki/X86-64#Microarchitecture_levels).

### A note for macOS users

macOS blocks unsigned binaries by default. Apple wants me to pay $99/year to sign the binary so you can avoid typing this command. So instead, after downloading, just run:

```bash
xattr -d com.apple.quarantine ~/Downloads/grail-arm64
```

and you should be able to run it! 🍎

### Configuration

Once added to your GUI, you can configure Grail via the UCI options:

- **Hash**: Size of the transposition table in MB (Default: 256).
- **Threads**: Number of search threads (Default: 1).
- **MultiPV**: Number of principal variations to search (Default: 1).
- **Move Overhead**: Time buffer in milliseconds to account for communication lag (Default: 10).
- **SyzygyPath**: Paths to Syzygy tablebase files (separated by `;` on Windows, `:` on Linux/macOS).
- **SyzygyProbeDepth**: Minimum depth to probe tablebases (Default: 1).

The engine supports standard time controls (increment, sudden death, moves to go) and analysis modes (fixed depth, fixed nodes, soft nodes, infinite).

## Play Against Grail Online

You can challenge the latest version of Grail on [Lichess](https://lichess.org/@/grail-bot), running on a 1 vCPU Northflank instance with 256 MB hash.

## For Developers

### Building from Source

_Grail requires the Rust nightly toolchain (for `portable_simd` and `generic_const_exprs`)!_

```bash
git clone https://github.com/jorgenhanssen/grail.git
cd grail
rustup override set nightly
make grail
```

The resulting release binary is written to `target/release/grail`.

### Build Targets

The project includes a `Makefile` for convenience:

- **`make` or `make grail`**: Release build
- **`make grail-pgo`**: Release build with PGO.
- **`make generate`**: Builds the NNUE self-play datagen.
- **`make generate-pgo`**: Builds the NNUE self-play datagen with PGO.
- **`make train`**: Builds the NNUE trainer (auto-detects CUDA/Metal).
- **`make nnue-analysis`**: Dumps a analysis of the current NNUE to `nnue/model.analysis.txt`.
- **`make profile`**: Profiles the built-in benchmark with [`samply`](https://github.com/mstange/samply).
- **`make clean`**: Remove the build directory.

### NNUE Data Generation & Training

Everything needed to generate self-play data and train Grail's NNUE lives in this repository.

#### Data Generation

Build the generator and choose either an EPD opening book or random moves from startpos:

```bash
make generate

# Openings from an EPD opening book
./target/release/generate book --path books/your_opening_book.epd

# Openings from startpos + random moves
./target/release/generate random --plies 8
```

**Arguments:**

- `--depth`: Search depth for each move (default: 8).
- `--nodes`: Soft node limit for each move.
- `--pv-lines`: Number of PV lines to search at each decision point (default: 1).
- `--threads`: Number of threads (default: number of logical CPUs).
- `--syzygy-path`: Paths to Syzygy tablebase files.
- `--max-opening-imbalance`: Discard games whose opening eval exceeds this many centipawns in absolute value.
- `--max-teleport-plies`: Max plies to teleport along a PV between recorded positions (default: 8).
- `--max-game-plies`: Discard games lasting longer than this many moves (default: 300).
- `--max-games`: Stop after this many games total.
- `--dry-run`: Generate samples but don't write the dataset to disk.

Generated data is saved to `nnue/data/YYYY-MM-DD-HH:MM.csv`.

#### Training

Train a new network using the generated data:

```bash
make train

./target/release/train
```

Run it again later and it'll pick up where it left off. Pass `--restart` to start from scratch.

**Arguments:**

- `--batch-size`: Batch size for training (default: 8192).
- `--learning-rate`: Initial learning rate (default: 0.001).
- `--epochs`: Max number of epochs to train (default: 200).
- `--workers`: Number of worker threads for data loading (default: 4).
- `--val-ratio`: Fraction of data to use for validation (default: 0.05).
- `--test-ratio`: Fraction of data to use for testing (default: 0.01).
- `--lr-decay`: Learning rate decay factor (default: 0.95).
- `--patience`: Epochs without improvement before early stopping (default: 5).
- `--shard-size-mb`: Size of each data shard in megabytes (default: 500).
- `--wdl`: WDL blending weight, 0.0 = pure eval, 1.0 = pure WDL (default: 0.3).
- `--draw-target`: Target win-probability for drawn games, smaller = prefer wins over draws (default: 0.5).
- `--restart`: Discard saved progress and train from epoch 1.

If you want to initialize a new/random model without starting training:

```bash
./target/release/train init
```

## Acknowledgements

- [Chess Programming Wiki](https://www.chessprogramming.org/) - A very helpful resource for chess programming concepts and techniques.
- Thanks to various engine testers, such as the [CCRL](https://computerchess.org.uk/ccrl/), for testing and ranking Grail.
- Opening books in `/books` sourced from the [Stockfish opening books](https://github.com/official-stockfish/books) and the computer chess community.
