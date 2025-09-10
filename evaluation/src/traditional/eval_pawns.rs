use chess::{get_adjacent_files, get_file, BitBoard, Board, Color, Piece, ALL_FILES, EMPTY};

use crate::traditional::bonus::PASSED_PAWN_BONUS;

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color) -> i16 {
    let my_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    if my_pawns == EMPTY {
        return 0;
    }

    let enemy_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);
    let mut score = 0i16;

    // doubled / tripled / isolated penalties
    for file_idx in ALL_FILES {
        let pawns_in_file = my_pawns & get_file(file_idx);
        let cnt = pawns_in_file.popcnt();
        if cnt == 0 {
            continue;
        }

        score -= match cnt {
            1 => {
                if (my_pawns & get_adjacent_files(file_idx)).popcnt() == 0 {
                    40 // Isolated
                } else {
                    0
                }
            }
            2 => 30, // Doubled
            _ => 60, // Tripled+
        };
    }

    // passed-pawn bonus
    for sq in my_pawns {
        let blockers = PASSED_PAWN_MASKS[color as usize][sq.to_index()];
        if (enemy_pawns & blockers).popcnt() == 0 {
            // convert to white’s perspective: rank 0..7
            let rank_from_white = if color == Color::White {
                sq.get_rank() as usize
            } else {
                7 - sq.get_rank() as usize
            };
            score += PASSED_PAWN_BONUS[rank_from_white];
        }
    }

    score
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
