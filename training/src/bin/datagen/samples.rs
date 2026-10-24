use cozy_chess::{Board, Color, Move};
use nnue::network::EVAL_OUTPUT_BOUND;
use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::{Arc, Mutex};

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

pub struct GameData {
    pub start_position: Board,
    pub moves: Vec<MoveData>,
}

pub struct MoveData {
    pub mv: Move,
    pub sample: Option<Sample>,
}

pub struct Sample {
    pub score: i16,
    pub outcome: GameOutcome,
}

pub type SampleWriter = Arc<Mutex<BufWriter<File>>>;

pub const CSV_HEADER: &str = "position,move,score,outcome";

pub fn write_game<W: Write>(writer: &mut W, game: &GameData) -> io::Result<()> {
    if game.moves.is_empty() {
        return Ok(());
    }

    // each game starts with the opening
    writeln!(writer, "{:#},,,", game.start_position)?;

    for data in &game.moves {
        let mv = data.mv;
        let mut score = String::new();
        let mut outcome = String::new();

        if let Some(sample) = &data.sample {
            score = sample
                .score
                .clamp(-EVAL_OUTPUT_BOUND, EVAL_OUTPUT_BOUND)
                .to_string();
            outcome = sample.outcome.to_string();
        }

        // NB: omits the position (these are moves)
        writeln!(writer, ",{mv},{score},{outcome}")?;
    }

    writer.flush()
}
