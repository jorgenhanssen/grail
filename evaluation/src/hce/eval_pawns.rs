use super::HCEConfig;
use crate::hce::context::EvalContext;
use arrayvec::ArrayVec;
use chess::{
    get_adjacent_files, get_file, get_pawn_attacks, BitBoard, Color, Rank, Square, ALL_FILES, EMPTY,
};

#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let pawns = board.pieces(chess::Piece::Pawn);
    let my_pawns = pawns & board.color_combined(color);
    if my_pawns == EMPTY {
        return 0;
    }

    let enemy_pawns = pawns & board.color_combined(!color);
    let mut score = 0i16;

    // doubled / tripled / isolated penalties
    for file_idx in ALL_FILES {
        let pawns_in_file = my_pawns & get_file(file_idx);
        let cnt = pawns_in_file.popcnt();
        if cnt == 0 {
            continue;
        }

        // Penalty for doubled / tripled pawns
        match cnt {
            1 => score -= 0,                           // Good case: no penalty
            2 => score -= config.doubled_pawn_penalty, // Bad case: doubled
            _ => score -= config.tripled_pawn_penalty, // Bad case: > tripled
        };

        // Isolated penalty
        if (my_pawns & get_adjacent_files(file_idx)).popcnt() == 0 {
            score -= config.isolated_pawn_penalty;
        }
    }

    // passed-pawn bonus with diminishing returns for multiple passed pawns
    let mut passed_pawn_bonuses = ArrayVec::<i16, 8>::new();
    for sq in my_pawns {
        // Passed pawn bonus
        let blockers = PASSED_PAWN_MASKS[color as usize][sq.to_index()];
        if (enemy_pawns & blockers).popcnt() == 0 {
            // convert to white's perspective: rank 0..7
            let rank_from_white = if color == Color::White {
                sq.get_rank() as i16
            } else {
                7 - sq.get_rank() as i16
            };
            // Formula: bonus = linear * (rank-1) + quadratic * (rank-1)^2
            let effective_rank = rank_from_white - 1; // 0-6 range
            if effective_rank > 0 {
                // Skip rank 0 (no bonus)
                let bonus = config.passed_pawn_linear * effective_rank
                    + config.passed_pawn_quadratic * effective_rank * effective_rank;
                passed_pawn_bonuses.push(bonus);
            }
        }
    }

    // Sort passed pawns by bonus (most advanced first) and apply diminishing returns
    // First pawn gets full bonus, second gets bonus/2, third gets bonus/3, etc.
    passed_pawn_bonuses.sort_unstable_by(|a, b| b.cmp(a));
    for (idx, bonus) in passed_pawn_bonuses.iter().enumerate() {
        score += bonus / (idx as i16 + 1);
    }

    // backward pawn penalty
    for sq in my_pawns {
        // Backward pawn penalty
        if is_backward_pawn(sq, color, my_pawns, enemy_pawns) {
            score -= config.backward_pawn_penalty;

            // Extra penalty if on a half-open file (no enemy pawns to block it)
            let file_mask = get_file(sq.get_file());
            if (enemy_pawns & file_mask).popcnt() == 0 {
                score -= config.backward_pawn_half_open_penalty;
            }
        }
    }

    score
}

// Check if a pawn is backward (https://www.chessprogramming.org/Backward_Pawn)
//
// A backward pawn is a positional weakness defined by:
// 1. Behind ALL friendly pawns on adjacent files (no pawn can defend it)
// 2. Stop square (one square ahead) is unsafe to push to
#[inline(always)]
fn is_backward_pawn(sq: Square, color: Color, my_pawns: BitBoard, enemy_pawns: BitBoard) -> bool {
    let file = sq.get_file();
    let rank = sq.get_rank();

    let friendly_adjacent_pawns = my_pawns & get_adjacent_files(file);

    if friendly_adjacent_pawns == EMPTY {
        // No pawns on adjacent files - not backward, just isolated
        return false;
    }

    // Check if any adjacent pawn is behind or level with this pawn
    for adjacent_pawn in friendly_adjacent_pawns {
        let adjacent_rank = adjacent_pawn.get_rank();
        let is_behind_or_level = match color {
            Color::White => adjacent_rank.to_index() <= rank.to_index(),
            Color::Black => adjacent_rank.to_index() >= rank.to_index(),
        };

        if is_behind_or_level {
            // Found a pawn that could potentially support - not backward
            return false;
        }
    }

    // Check if stop square is safely pushable
    let stop_rank = match color {
        Color::White if rank.to_index() < 7 => Rank::from_index(rank.to_index() + 1),
        Color::Black if rank.to_index() > 0 => Rank::from_index(rank.to_index() - 1),
        _ => return false, // Can't move forward
    };

    // We use `color` here because we want the forward-diagonal attack squares
    // from the stop square relative to our pawn. Those are exactly the squares
    // enemy pawns must occupy to attack that stop square.
    if get_pawn_attacks(Square::make_square(stop_rank, file), color, enemy_pawns) != EMPTY {
        // Enemy pawns control the stop square - definitely backward
        return true;
    }

    // Behind friendly pawns but stop square not attacked by enemy pawns.
    // Still backward because it lacks pawn support to push safely.
    true
}

/// Pre-computed passed-pawn masks: [color][square].
pub const PASSED_PAWN_MASKS: [[BitBoard; 64]; 2] = {
    let mut table = [[BitBoard(0); 64]; 2];
    let mut square_idx = 0;
    while square_idx < 64 {
        let file_idx = (square_idx % 8) as i8;
        let rank_idx = (square_idx / 8) as i8;

        table[Color::White as usize][square_idx] =
            BitBoard(make_passed_pawn_mask(rank_idx, file_idx, 1));
        table[Color::Black as usize][square_idx] =
            BitBoard(make_passed_pawn_mask(rank_idx, file_idx, -1));

        square_idx += 1;
    }
    table
};

/// Bit-mask of every square that must be free of enemy pawns
/// for the pawn on (rank_idx, file_idx) to be counted as passed.
const fn make_passed_pawn_mask(
    mut rank_idx: i8, // starting rank of the pawn
    file_idx: i8,     // starting file of the pawn
    step: i8,         // +1 for white, -1 for black
) -> u64 {
    let mut mask = 0u64;
    rank_idx += step; // start one rank in front
    while rank_idx >= 0 && rank_idx < 8 {
        // stay on the board
        let mut scan_file = file_idx - 1; // current file in the -1..+1 window
        while scan_file <= file_idx + 1 {
            if scan_file >= 0 && scan_file < 8 {
                mask |= 1u64 << ((rank_idx as u64) * 8 + scan_file as u64);
            }
            scan_file += 1;
        }
        rank_idx += step; // move window one rank forward
    }
    mask
}
