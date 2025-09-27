use chess::{Board, ChessMove, Piece};
use evaluation::piece_values::PieceValues;

#[inline]
pub fn see(board: &Board, mv: ChessMove, phase: f32, piece_values: &PieceValues) -> i16 {
    let target = mv.get_dest();

    let mut initial_gain: i16 = if let Some(victim) = board.piece_on(target) {
        piece_values.get(victim, phase)
    } else {
        0
    };

    // Account for promotion delta
    if let Some(promo) = mv.get_promotion() {
        initial_gain += piece_values.get(promo, phase) - piece_values.get(Piece::Pawn, phase);
    }

    // Gains list stores the net gain after each ply using the CPW swaplist method
    let mut gains: Vec<i16> = Vec::with_capacity(8);
    gains.push(initial_gain);

    // Simulate alternating recaptures choosing the least valuable attacker each time
    let mut current_board = board.make_move_new(mv);

    // Alternate sides capturing on target until no legal recapture exists
    loop {
        // Generate capture moves for this square
        let mut recaptures = chess::MoveGen::new_legal(&current_board);
        recaptures.set_iterator_mask(chess::BitBoard::from_square(target));

        // Choose the least valuable attacker among recaptures
        let mut best_recapture = None;
        let mut best_value = i16::MAX;
        for mov in recaptures {
            if let Some(attacker) = current_board.piece_on(mov.get_source()) {
                let val = piece_values.get(attacker, phase);
                if val < best_value {
                    best_value = val;
                    best_recapture = Some(mov);
                }
            }
        }

        match best_recapture {
            Some(best) => {
                // Forward pass: net gain at this ply is captured piece value minus previous net
                let prev = *gains.last().unwrap();
                let captured_piece = current_board
                    .piece_on(target)
                    .expect("target must be occupied before recapture");
                let captured_value = piece_values.get(captured_piece, phase);
                gains.push(captured_value - prev);
                current_board = current_board.make_move_new(best);
            }
            None => break,
        }
    }

    // Backward induction (CPW): gains[i-1] = -max(-gains[i-1], gains[i])
    let mut i = gains.len();
    while i > 1 {
        let next = gains[i - 1];
        let prev = gains[i - 2];
        gains[i - 2] = -std::cmp::max(-prev, next);
        i -= 1;
    }

    gains[0]
}
