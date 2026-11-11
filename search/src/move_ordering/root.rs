use cozy_chess::Move;
use utils::move_vec_item;

/// Moves and order for the root across depths and aspiration retries.
#[derive(Default)]
pub struct RootMoves {
    moves: Vec<Move>,
    start: usize,
    idx: usize,
}

impl RootMoves {
    pub fn new(moves: Vec<Move>) -> Self {
        Self {
            moves,
            start: 0,
            idx: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn restart(&mut self, start: usize) {
        self.start = start;
        self.idx = start;
    }

    pub fn next(&mut self) -> Option<Move> {
        let mv = self.moves.get(self.idx).copied()?;
        self.idx += 1;

        Some(mv)
    }

    pub fn promote(&mut self, mv: Move) {
        let idx = self.moves.iter().position(|&m| m == mv).unwrap();
        move_vec_item(&mut self.moves, idx, self.start);
    }
}
