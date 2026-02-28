use cozy_chess::{Color, Move};
use search::PvLine;

pub struct SearchResultMeta {
    lines: Vec<PvLine>,
    stm: Color,
}

impl SearchResultMeta {
    pub fn new(lines: Vec<PvLine>, stm: Color) -> Self {
        Self { lines, stm }
    }

    pub fn top_moves(&self) -> Vec<Move> {
        self.lines.iter().filter_map(|pv| pv.best_move()).collect()
    }

    pub fn scores_white(&self) -> impl Iterator<Item = i16> + '_ {
        let sign = if self.stm == Color::White { 1 } else { -1 };
        self.lines.iter().map(move |pv| pv.score * sign)
    }
}
