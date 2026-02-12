use nnue::network::CP_BOUND;
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub struct Samples {
    pub fens: Vec<Box<str>>,
    pub scores: Vec<i16>,
    pub moves: Vec<Box<str>>,
    pub game_ids: Vec<usize>,
}

impl Samples {
    pub fn from_evaluations(evals: &[(String, i16, String, usize)]) -> Self {
        let mut fens = Vec::with_capacity(evals.len());
        let mut scores = Vec::with_capacity(evals.len());
        let mut moves = Vec::with_capacity(evals.len());
        let mut game_ids = Vec::with_capacity(evals.len());
        for (fen, score, mv, game_id) in evals.iter() {
            fens.push(fen.clone().into_boxed_str());
            scores.push((*score).clamp(-CP_BOUND, CP_BOUND));
            moves.push(mv.clone().into_boxed_str());
            game_ids.push(*game_id);
        }
        Self {
            fens,
            scores,
            moves,
            game_ids,
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "fen,score,move,game_id")?;

        for i in 0..self.fens.len() {
            writeln!(
                writer,
                "{},{},{},{}",
                self.fens[i], self.scores[i], self.moves[i], self.game_ids[i]
            )?;
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.fens.len()
    }
}
