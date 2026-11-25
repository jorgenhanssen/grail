use cozy_chess::{Board, Color, Move};
use std::fmt;

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

pub struct TrainingSample {
    pub board: Board,
    pub score: i16,
    pub outcome: GameOutcome,
}

impl GameData {
    pub fn into_samples(self) -> impl Iterator<Item = TrainingSample> {
        let mut board = self.start_position;

        self.moves.into_iter().filter_map(move |data| {
            let sample = data.sample.map(|sample| TrainingSample {
                board: board.clone(),
                score: sample.score,
                outcome: sample.outcome,
            });

            board.play_unchecked(data.mv);
            sample
        })
    }
}

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
