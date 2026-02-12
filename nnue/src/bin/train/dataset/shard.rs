use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use cozy_chess::{Board, Color, util::parse_uci_move};
use nnue::encoding::{NUM_FEATURES, encode_board};
use nnue::network::{FV_SCALE, output_bucket};
use utils::board_metrics::BoardMetrics;

/// A single sample from a shard file.
#[derive(Debug, Clone)]
pub struct Sample {
    pub fen: String,
    pub score: i16,
    pub best_move: String,
}

/// Sample encoded as tensors for training.
pub struct EncodedSample {
    pub features: [f32; NUM_FEATURES],
    pub score: f32,
    pub bucket: usize,
    pub policy_target: usize,
}

impl Sample {
    pub fn encode(&self) -> Option<EncodedSample> {
        let board = Board::from_str(&self.fen).ok()?;
        let metrics = BoardMetrics::new(&board);

        let score = self.score as f32 / FV_SCALE;
        let bucket = output_bucket(&board);
        let features = encode_board(
            &board,
            metrics.attacks[Color::White as usize],
            metrics.attacks[Color::Black as usize],
            metrics.support[Color::White as usize],
            metrics.support[Color::Black as usize],
            metrics.threats[Color::White as usize],
            metrics.threats[Color::Black as usize],
        );

        let mv = parse_uci_move(&board, &self.best_move).ok()?;
        let piece = board.piece_on(mv.from)?;
        let policy_target = piece as usize;

        Some(EncodedSample {
            score,
            bucket,
            features,
            policy_target,
        })
    }
}

/// Reads samples sequentially from a single CSV shard file.
pub struct Shard {
    reader: BufReader<File>,
}

impl Shard {
    /// Opens a shard file for reading.
    /// Skips the header line (fen,score).
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Skip header line
        let mut header = String::new();
        reader.read_line(&mut header)?;

        Ok(Self { reader })
    }
}

impl Iterator for Shard {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let mut line = String::new();

        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => parse_line(&line),
            Err(_) => None,
        }
    }
}

fn parse_line(line: &str) -> Option<Sample> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut parts = line.split(',');
    let fen = parts.next()?.to_string();
    let score: i16 = parts.next()?.parse().ok()?;
    let best_move = parts.next()?.to_string();

    Some(Sample {
        fen,
        score,
        best_move,
    })
}
