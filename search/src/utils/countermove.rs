use chess::{Board, ChessMove, NUM_COLORS, NUM_SQUARES};

#[derive(Clone)]
pub struct CountermoveTable {
    table: [[[Option<ChessMove>; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
}

impl CountermoveTable {
    pub fn new() -> Self {
        Self {
            table: [[[None; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.table = [[[None; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
    }

    #[inline(always)]
    pub fn get(&self, board: &Board, move_stack: &[ChessMove]) -> Option<ChessMove> {
        let parent = move_stack.last()?;
        self.table[board.side_to_move().to_index()][parent.get_source().to_index()]
            [parent.get_dest().to_index()]
    }

    #[inline(always)]
    pub fn store(&mut self, board: &Board, move_stack: &[ChessMove], reply: ChessMove) {
        if let Some(parent) = move_stack.last() {
            self.table[board.side_to_move().to_index()][parent.get_source().to_index()]
                [parent.get_dest().to_index()] = Some(reply);
        }
    }
}
