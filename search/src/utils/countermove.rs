use chess::{Board, ChessMove, NUM_COLORS, NUM_SQUARES};

const DEFAULT_MOVE_TTL: u16 = 8;

#[derive(Copy, Clone)]
struct CountermoveEntry {
    mv: ChessMove,
    stored_at_move: u16,
}

#[derive(Clone)]
pub struct CountermoveTable {
    table: [[[Option<CountermoveEntry>; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
    move_ttl: u16,
    current_move: u16,
}

impl CountermoveTable {
    pub fn new() -> Self {
        Self {
            table: [[[None; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
            move_ttl: DEFAULT_MOVE_TTL,
            current_move: 0,
        }
    }

    #[inline(always)]
    pub fn on_new_position(&mut self) {
        self.current_move = self.current_move.wrapping_add(1);
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.table = [[[None; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
    }

    #[inline(always)]
    pub fn get(&self, board: &Board, move_stack: &[ChessMove]) -> Option<ChessMove> {
        let parent = move_stack.last()?;
        let entry = self.table[board.side_to_move().to_index()][parent.get_source().to_index()]
            [parent.get_dest().to_index()];

        match entry {
            Some(e) if self.current_move.wrapping_sub(e.stored_at_move) <= self.move_ttl => {
                Some(e.mv)
            }
            _ => None,
        }
    }

    #[inline(always)]
    pub fn store(&mut self, board: &Board, move_stack: &[ChessMove], reply: ChessMove) {
        if let Some(parent) = move_stack.last() {
            self.table[board.side_to_move().to_index()][parent.get_source().to_index()]
                [parent.get_dest().to_index()] = Some(CountermoveEntry {
                mv: reply,
                stored_at_move: self.current_move,
            });
        }
    }
}
