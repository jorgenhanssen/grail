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
