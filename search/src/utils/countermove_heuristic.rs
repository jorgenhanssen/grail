use chess::{Board, ChessMove, Color, Piece, Square, NUM_COLORS, NUM_PIECES, NUM_SQUARES};

#[derive(Clone)]
pub struct CountermoveHeuristic {
    countermove: [[[Option<ChessMove>; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS],
}

impl CountermoveHeuristic {
    pub fn new() -> Self {
        Self {
            countermove: [[[None; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.countermove = [[[None; NUM_SQUARES]; NUM_PIECES]; NUM_COLORS];
    }

    #[inline(always)]
    fn get_move(&self, color: Color, piece: Piece, square: Square) -> Option<ChessMove> {
        self.countermove[color.to_index()][piece.to_index()][square.to_index()]
    }

    #[inline(always)]
    fn set_move(&mut self, color: Color, piece: Piece, square: Square, mov: ChessMove) {
        self.countermove[color.to_index()][piece.to_index()][square.to_index()] = Some(mov);
    }

    // Get the countermove for the current position based on the previous move
    #[inline(always)]
    pub fn get(&self, board: &Board, prev_move: Option<ChessMove>) -> Option<ChessMove> {
        prev_move.and_then(|lm| {
            let prev_to = lm.get_dest();
            if let Some(prev_piece) = board.piece_on(prev_to) {
                let current_color = board.side_to_move();
                self.get_move(current_color, prev_piece, prev_to)
            } else {
                None
            }
        })
    }

    // Update the countermove table with a good response to the previous move
    #[inline(always)]
    pub fn update(
        &mut self,
        board: &Board,
        prev_move: Option<ChessMove>,
        response_move: ChessMove,
    ) {
        if let Some(prev_move) = prev_move {
            let prev_to = prev_move.get_dest();
            if let Some(prev_piece) = board.piece_on(prev_to) {
                let current_color = board.side_to_move();
                self.set_move(current_color, prev_piece, prev_to, response_move);
            }
        }
    }
}
