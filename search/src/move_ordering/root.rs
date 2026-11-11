use cozy_chess::Move;
use utils::move_vec_item;

/// RootMoves keeps an explicit list of root moves in search order.
///
/// Should be initialized by the normal move ordering. When a move beats alpha, it moves to
/// the front of the part of the list being searched so we try it first next time.
///
/// For MultiPV, each PV starts one slot further into the list, skipping moves
/// already chosen for earlier PVs. Restarting resets the cursor but keeps the order.
#[derive(Default)]
pub struct RootMoves {
    moves: Vec<Move>,
    current_pv_start: usize,
    move_index: usize,
}

impl RootMoves {
    pub fn new(moves: Vec<Move>) -> Self {
        Self {
            moves,
            current_pv_start: 0,
            move_index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn restart(&mut self, pv_start: usize) {
        self.current_pv_start = pv_start;
        self.move_index = pv_start;
    }

    pub fn next(&mut self) -> Option<Move> {
        let mv = self.moves.get(self.move_index).copied()?;
        self.move_index += 1;

        Some(mv)
    }

    pub fn promote(&mut self, mv: Move) {
        let move_index = self.moves.iter().position(|&m| m == mv).unwrap();
        move_vec_item(&mut self.moves, move_index, self.current_pv_start);
    }
}
