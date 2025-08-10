use chess::{Board, ChessMove, Piece};
use evaluation::piece_value;

#[inline]
pub fn see(board: &Board, mv: ChessMove, phase: f32) -> i16 {
    let target = mv.get_dest();

    let mut initial_gain: i16 = if let Some(victim) = board.piece_on(target) {
        piece_value(victim, phase)
    } else {
        0
    };

    // Account for promotion delta
    if let Some(promo) = mv.get_promotion() {
        initial_gain += piece_value(promo, phase) - piece_value(Piece::Pawn, phase);
    }

    // Gains list stores the captured value at each ply
    let mut gains: Vec<i16> = Vec::with_capacity(16);
    gains.push(initial_gain);

    // Simulate alternating recaptures choosing the least valuable attacker each time
    let mut current_board = board.make_move_new(mv);

    // Alternate sides capturing on target until no legal recapture exists
    loop {
        // Find all legal recaptures to target square
        let recaptures = chess::MoveGen::new_legal(&current_board)
            .filter(|m| m.get_dest() == target)
            .collect::<Vec<_>>();

        if recaptures.is_empty() {
            break;
        }

        // Choose the least valuable attacker to continue the sequence
        let mut best_recapture = None;
        let mut best_value = i16::MAX;
        for m in recaptures {
            if let Some(attacker) = current_board.piece_on(m.get_source()) {
                let val = piece_value(attacker, phase);
                if val < best_value {
                    best_value = val;
                    best_recapture = Some(m);
                }
            }
        }

        if let Some(best) = best_recapture {
            gains.push(best_value);
            current_board = current_board.make_move_new(best);
        } else {
            break;
        }
    }

    // Backward induction to compute optimal outcome for the initial side
    // Standard SEE fold: gains[i-1] = max(0, gains[i-1] - gains[i])
    let mut i = gains.len();
    while i > 1 {
        let next = gains[i - 1];
        let prev = gains[i - 2];
        let new_prev = prev - next;
        gains[i - 2] = new_prev.max(0);
        i -= 1;
    }

    gains[0]
}
