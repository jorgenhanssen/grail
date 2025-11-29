use cozy_chess::{Board, Move, Piece};
use evaluation::piece_values::PieceValues;
use utils::make_move;

/// Evaluates material gain/loss from an exhaustive capture sequence on a square.
///
/// Builds a swaplist of alternating recaptures (least valuable attacker first),
/// then works backward letting each side choose: recapture or stop.
///
/// <https://www.chessprogramming.org/Static_Exchange_Evaluation>
#[inline]
pub fn see(board: &Board, mv: Move, phase: f32, piece_values: &PieceValues) -> i16 {
    let target = mv.to;
    let target_bb = target.bitboard();

    let mut initial_gain: i16 = if let Some(victim) = board.piece_on(target) {
        piece_values.get(victim, phase)
    } else {
        0
    };

    // Account for promotion delta
    if let Some(promo) = mv.promotion {
        initial_gain += piece_values.get(promo, phase) - piece_values.get(Piece::Pawn, phase);
    }

    // Gains list stores the net gain after each ply using the CPW swaplist method.
    // 16 max captures should be safe.
    let mut gains: [i16; 16] = [0; 16];
    let mut gains_length = 1;
    gains[0] = initial_gain;

    // Simulate alternating recaptures choosing the least valuable attacker each time
    let mut current_board = make_move(board, mv);

    // Alternate sides capturing on target until no legal recapture exists
    loop {
        // Choose the least valuable attacker among recaptures
        let mut best_recapture: Option<Move> = None;
        let mut best_value = i16::MAX;

        current_board.generate_moves(|moves| {
            for mov in moves {
                if !target_bb.has(mov.to) {
                    continue;
                }
                if let Some(attacker) = current_board.piece_on(mov.from) {
                    let val = piece_values.get(attacker, phase);
                    if val < best_value {
                        best_value = val;
                        best_recapture = Some(mov);
                    }
                }
            }
            false
        });

        match best_recapture {
            Some(best) => {
                // Forward pass: net gain at this ply is captured piece value minus previous net
                let prev = gains[gains_length - 1];
                let captured_piece = current_board
                    .piece_on(target)
                    .expect("target must be occupied before recapture");
                let captured_value = piece_values.get(captured_piece, phase);

                gains[gains_length] = captured_value - prev;
                gains_length += 1;
                current_board.play_unchecked(best);
            }
            None => break,
        }
    }

    // Backward induction: each side chooses max(current gain, -opponent's best).
    // Must evaluate from the end since earlier decisions depend on later outcomes.
    let mut i = gains_length;
    while i > 1 {
        let next = gains[i - 1];
        let prev = gains[i - 2];
        gains[i - 2] = -std::cmp::max(-prev, next);
        i -= 1;
    }

    gains[0]
}
