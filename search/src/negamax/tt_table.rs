use ahash::AHashMap;
use chess::ChessMove;

#[derive(Clone, Copy, PartialEq)]
pub enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub plies: u8,
    pub value: i16,
    pub best_move: Option<ChessMove>,
    pub bound: Bound,
}

impl TTEntry {
    #[inline(always)]
    pub fn new(plies: u8, value: i16, bound: Bound, best_move: Option<ChessMove>) -> Self {
        Self {
            plies,
            value,
            best_move,
            bound,
        }
    }
}

pub struct TranspositionTable {
    map: AHashMap<u64, TTEntry>,
}

impl TranspositionTable {
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: AHashMap::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline(always)]
    pub fn probe(
        &self,
        hash: u64,
        depth: u8,
        max_depth: u8,
    ) -> Option<(i16, Bound, Option<ChessMove>)> {
        let plies = max_depth - depth;
        if let Some(entry) = self.map.get(&hash) {
            if entry.plies >= plies {
                return Some((entry.value, entry.bound, entry.best_move));
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn store(
        &mut self,
        hash: u64,
        depth: u8,
        max_depth: u8,
        value: i16,
        alpha: i16,
        beta: i16,
        best_move: Option<ChessMove>,
    ) {
        let plies = max_depth - depth;

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        let entry = TTEntry::new(plies, value, bound, best_move);

        if let Some(old_entry) = self.map.get(&hash) {
            if old_entry.plies <= plies {
                self.map.insert(hash, entry);
            }
        } else {
            self.map.insert(hash, entry);
        }
    }
}
