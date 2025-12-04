use cozy_chess::{Board, Move, Piece, Square};
use evaluation::piece_values::PieceValues;
use utils::{get_attackers_to, get_discovered_attacks};

/// Static Exchange Evaluation (SEE) with threshold comparison.
///
/// Evaluates the material outcome of a capture sequence on a square without
/// making any moves. Used extensively for move ordering and pruning decisions.
///
/// Returns true if the SEE value >= threshold. This is the de facto standard
/// approach in modern chess engines, as it enables early exits and avoids
/// computing the full SEE value when only a comparison is needed.
///
/// # Algorithm
///
/// Uses the negamax swap algorithm with bitboard manipulation:
/// 1. Early exit if capturing for free doesn't meet threshold
/// 2. Early exit if we still meet threshold after losing our piece
/// 3. Otherwise, simulate the capture sequence using bitboards (no moves made)
/// 4. Use `gain = -gain - 1 - piece_value` (negamax with strict inequality)
/// 5. Handle X-ray attacks by updating occupancy and recalculating sliders
///
/// # References
///
/// - <https://www.chessprogramming.org/Static_Exchange_Evaluation>
/// - [Stockfish `see_ge`](https://github.com/official-stockfish/Stockfish/blob/master/src/position.h)
/// - [Black Marlin](https://github.com/jnlt3/blackmarlin)
/// - [PlentyChess](https://github.com/Yoshie2000/PlentyChess)
#[inline]
pub fn see(
    board: &Board,
    mv: Move,
    phase: f32,
    piece_values: &PieceValues,
    threshold: i16,
) -> bool {
    let from = mv.from;
    let to = mv.to;

    // Initial gain: value of victim - threshold
    let mut gain = board
        .piece_on(to)
        .map(|victim| piece_values.get(victim, phase))
        .unwrap_or(0)
        - threshold;

    // Account for promotion delta
    if let Some(promo) = mv.promotion {
        gain += piece_values.get(promo, phase) - piece_values.get(Piece::Pawn, phase);
    }

    // If taking the victim for free isn't enough, fail immediately
    if gain < 0 {
        return false;
    }

    // Subtract the value of our attacker (they might recapture it)
    let attacker = board.piece_on(from).unwrap_or(Piece::Pawn);
    gain -= piece_values.get(attacker, phase);

    // If we're still above threshold after potentially losing our attacker, succeed
    // (we can always choose to stop if recapturing would be bad)
    if gain >= 0 {
        return true;
    }

    // Set up occupancy for X-ray handling
    let mut occupied = board.occupied();
    occupied ^= from.bitboard();
    occupied |= to.bitboard();

    // Calculate all attackers to the target square
    let mut attackers = get_attackers_to(board, to, occupied);

    let bishops_queens = board.pieces(Piece::Bishop) | board.pieces(Piece::Queen);
    let rooks_queens = board.pieces(Piece::Rook) | board.pieces(Piece::Queen);

    let mut stm = !board.side_to_move();

    loop {
        // Filter out pieces that have already captured
        attackers &= occupied;

        let my_attackers = attackers & board.colors(stm);
        if my_attackers.is_empty() {
            break;
        }

        // Find least valuable attacker
        let (piece, attacker_sq) = find_least_valuable_attacker(board, my_attackers);

        // Switch sides
        stm = !stm;

        // Negamax formula: -gain - 1 - piece_value
        // The -1 handles strict inequality in integer domain
        gain = -gain - 1 - piece_values.get(piece, phase);

        if gain >= 0 {
            // If king captured and opponent still has attackers, king capture is illegal
            if piece == Piece::King && !(attackers & board.colors(stm)).is_empty() {
                stm = !stm;
            }
            break;
        }

        // Remove the attacker from occupancy
        occupied ^= attacker_sq.bitboard();

        // Add any discovered attackers
        attackers |= get_discovered_attacks(piece, to, occupied, bishops_queens, rooks_queens);
    }

    // If we ended on our original side's turn, opponent had the last profitable capture
    stm != board.side_to_move()
}

/// Finds the least valuable attacker from the given attackers bitboard.
#[inline]
fn find_least_valuable_attacker(board: &Board, attackers: cozy_chess::BitBoard) -> (Piece, Square) {
    // Piece::ALL is ordered by value: Pawn, Knight, Bishop, Rook, Queen, King
    for piece in Piece::ALL {
        let piece_attackers = attackers & board.pieces(piece);
        if let Some(sq) = piece_attackers.next_square() {
            return (piece, sq);
        }
    }
    unreachable!("attackers bitboard should not be empty")
}
