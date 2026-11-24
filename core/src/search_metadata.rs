use cozy_chess::{Color, Move};
use search::{PvLine, SearchResult};

pub struct SearchResultMeta {
    result: SearchResult,
    stm: Color,
}

impl SearchResultMeta {
    pub fn new(result: SearchResult, stm: Color) -> Self {
        Self { result, stm }
    }

    pub fn top_moves(&self) -> Vec<Move> {
        self.result
            .lines()
            .iter()
            .filter_map(PvLine::best_move)
            .collect()
    }

    pub fn scores_white(&self) -> impl Iterator<Item = i16> + '_ {
        let sign = if self.stm == Color::White { 1 } else { -1 };
        self.result.lines().iter().map(move |pv| pv.score * sign)
    }
}
