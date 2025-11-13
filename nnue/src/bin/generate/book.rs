use rand::Rng;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const EPD_MIN_FIELDS: usize = 4;
const DEFAULT_HALFMOVE: u8 = 0;
const DEFAULT_FULLMOVE: u16 = 1;

pub struct Book {
    positions: Vec<String>,
}

impl Book {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);

        let mut positions = Vec::new();
        for line in reader.lines() {
            if let Some(fen) = parse_epd_line(&line?) {
                positions.push(fen);
            }
        }

        if positions.is_empty() {
            return Err("Opening book is empty".into());
        }

        log::info!("Loaded {} positions from opening book", positions.len());

        Ok(Self { positions })
    }

    pub fn random_position(&self) -> &str {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.positions.len());
        &self.positions[index]
    }
}

fn parse_epd_line(line: &str) -> Option<String> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    // EPD format: <board> <side> <castling> <ep> <operations...>
    // We extract just the first 4 fields and ignore EPD operations (c0, bm, etc.)
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= EPD_MIN_FIELDS {
        // Construct FEN from first 4 fields (board, side, castling, ep)
        // Add default halfmove clock and fullmove number for complete FEN
        Some(format!(
            "{} {} {} {} {} {}",
            parts[0], parts[1], parts[2], parts[3], DEFAULT_HALFMOVE, DEFAULT_FULLMOVE
        ))
    } else {
        None
    }
}
