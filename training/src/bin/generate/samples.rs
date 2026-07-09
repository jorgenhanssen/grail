use cozy_chess::{Color, Move};
use nnue::network::CP_BOUND;
use std::fmt;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    White,
    Draw,
    Black,
}

impl GameOutcome {
    pub fn win(color: Color) -> Self {
        match color {
            Color::White => GameOutcome::White,
            Color::Black => GameOutcome::Black,
        }
    }
}

impl fmt::Display for GameOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameOutcome::White => write!(f, "W"),
            GameOutcome::Draw => write!(f, "D"),
            GameOutcome::Black => write!(f, "B"),
        }
    }
}

pub struct Sample {
    pub fen: String,
    pub score: i16,
    pub game_id: usize,
    pub best_move: Move,
    pub outcome: GameOutcome,
}

pub fn write_samples<W: Write>(writer: &mut W, samples: &[Sample]) -> io::Result<()> {
    writeln!(writer, "fen,score,best_move,outcome,game_id")?;

    for s in samples {
        writeln!(
            writer,
            "{},{},{},{},{}",
            s.fen,
            s.score.clamp(-CP_BOUND, CP_BOUND),
            s.best_move,
            s.outcome,
            s.game_id,
        )?;
    }

    Ok(())
}
