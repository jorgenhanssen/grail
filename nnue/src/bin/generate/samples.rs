use nnue::network::{CP_MAX, CP_MIN};
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub struct Samples {
    pub fens: Vec<Box<str>>,
    pub scores: Vec<i16>,
    pub game_ids: Vec<usize>,
}

impl Samples {
    pub fn from_evaluations(evals: &[(String, i16, usize)]) -> Self {
        let mut fens = Vec::with_capacity(evals.len());
        let mut scores = Vec::with_capacity(evals.len());
        let mut game_ids = Vec::with_capacity(evals.len());
        for (fen, score, game_id) in evals.iter() {
            fens.push(fen.clone().into_boxed_str());
            scores.push((*score).clamp(CP_MIN, CP_MAX));
            game_ids.push(*game_id);
        }
        Self {
            fens,
            scores,
            game_ids,
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score,game_id")?; // Header

        for i in 0..self.fens.len() {
            writeln!(
                writer,
                "{},{},{}",
                self.fens[i], self.scores[i], self.game_ids[i]
            )?;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.fens.len()
    }
}
