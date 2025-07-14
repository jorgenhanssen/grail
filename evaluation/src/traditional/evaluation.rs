use chess::{
    get_adjacent_files, get_file, BitBoard, Board, BoardStatus, Color, Piece, Rank, ALL_FILES,
    EMPTY,
};

use crate::traditional::pst::{get_pst, sum_pst};
use crate::traditional::values::{
    piece_value, BISHOP_VALUE, KNIGHT_VALUE, PASSED_PAWN_BONUS, PAWN_VALUE, QUEEN_VALUE,
    ROOK_ON_SEVENTH_BONUS, ROOK_OPEN_FILE_BONUS, ROOK_SEMI_OPEN_FILE_BONUS, ROOK_VALUE,
};

// Return final evaluation (positive = good for White, negative = good for Black)
pub fn evaluate_board(board: &Board) -> f32 {
    let is_white = board.side_to_move() == Color::White;

    let phase = game_phase(board);

    match board.status() {
        BoardStatus::Checkmate => {
            // If it's White to move and board is checkmated => White lost
            if is_white {
                return -1.0;
            } else {
                return 1.0;
            }
        }
        BoardStatus::Stalemate => return 0.0,
        BoardStatus::Ongoing => {}
    }

    let white_mask = board.color_combined(Color::White);
    let black_mask = board.color_combined(Color::Black);

    let mut score = 0.0;
    score += evaluate_material(board, Color::White, &white_mask, phase);
    score -= evaluate_material(board, Color::Black, &black_mask, phase);

    score += evaluate_pawn_structure(board, Color::White);
    score -= evaluate_pawn_structure(board, Color::Black);

    score += evaluate_rooks(board, Color::White);
    score -= evaluate_rooks(board, Color::Black);

    score += evaluate_king_safety(board, Color::White);
    score -= evaluate_king_safety(board, Color::Black);

    (score / 1_500.0).tanh()
}

fn evaluate_material(board: &Board, color: Color, color_mask: &BitBoard, phase: f32) -> f32 {
    let pawn_mask = board.pieces(Piece::Pawn) & color_mask;
    let knight_mask = board.pieces(Piece::Knight) & color_mask;
    let bishop_mask = board.pieces(Piece::Bishop) & color_mask;
    let rook_mask = board.pieces(Piece::Rook) & color_mask;
    let queen_mask = board.pieces(Piece::Queen) & color_mask;
    let king_mask = board.pieces(Piece::King) & color_mask;

    let num_pawns = pawn_mask.popcnt();
    let num_knights = knight_mask.popcnt();
    let num_bishops = bishop_mask.popcnt();
    let num_rooks = rook_mask.popcnt();
    let num_queens = queen_mask.popcnt();

    let piece_value = PAWN_VALUE * num_pawns as f32
        + KNIGHT_VALUE * num_knights as f32
        + BISHOP_VALUE * num_bishops as f32
        + ROOK_VALUE * num_rooks as f32
        + QUEEN_VALUE * num_queens as f32;

    let pst = get_pst(color);
    let mut pst_value = 0.0;
    if num_pawns > 0 {
        pst_value += sum_pst(pawn_mask, pst.pawn, phase);
    }
    if num_knights > 0 {
        pst_value += sum_pst(knight_mask, pst.knight, phase);
    }
    if num_bishops > 0 {
        pst_value += sum_pst(bishop_mask, pst.bishop, phase);
    }
    if num_rooks > 0 {
        pst_value += sum_pst(rook_mask, pst.rook, phase);
    }
    if num_queens > 0 {
        pst_value += sum_pst(queen_mask, pst.queen, phase);
    }
    pst_value += sum_pst(king_mask, pst.king, phase);

    // bonus for bishop pair
    let bishop_pair_bonus = if num_bishops >= 2 { 50.0 } else { 0.0 };

    return piece_value + pst_value + bishop_pair_bonus;
}

fn evaluate_pawn_structure(board: &Board, color: Color) -> f32 {
    let my_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    if my_pawns == EMPTY {
        return 0.0;
    }

    let enemy_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);
    let mut score = 0.0;

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
                    30.0
                } else {
                    0.0
                }
            }
            2 => 20.0,
            _ => 40.0,
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

fn evaluate_king_safety(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let king_square = board.king_square(color);
    let king_zone = KING_ZONES[king_square.to_index()];
    let enemy_color = !color;
    let enemy_pieces = board.color_combined(enemy_color);

    // Weight threats by piece type
    let enemy_queens = enemy_pieces & board.pieces(Piece::Queen) & king_zone;
    let enemy_rooks = enemy_pieces & board.pieces(Piece::Rook) & king_zone;
    let enemy_bishops = enemy_pieces & board.pieces(Piece::Bishop) & king_zone;
    let enemy_knights = enemy_pieces & board.pieces(Piece::Knight) & king_zone;
    let enemy_pawns = enemy_pieces & board.pieces(Piece::Pawn) & king_zone;

    // Apply different penalties based on piece type
    score -= (enemy_queens.popcnt() as f32) * piece_value(Piece::Queen);
    score -= (enemy_rooks.popcnt() as f32) * piece_value(Piece::Rook);
    score -= (enemy_bishops.popcnt() as f32) * piece_value(Piece::Bishop);
    score -= (enemy_knights.popcnt() as f32) * piece_value(Piece::Knight);
    score -= (enemy_pawns.popcnt() as f32) * piece_value(Piece::Pawn);

    score * 0.3
}

const KING_ZONE_RADIUS: i8 = 2;
const KING_ZONES: [BitBoard; 64] = {
    let mut zones = [EMPTY; 64];
    let mut i = 0;
    while i < 64 {
        let king_file = (i % 8) as i8;
        let king_rank = (i / 8) as i8;

        let mut zone = EMPTY;
        let mut rank_offset = -KING_ZONE_RADIUS;
        while rank_offset <= KING_ZONE_RADIUS {
            let mut file_offset = -KING_ZONE_RADIUS;
            while file_offset <= KING_ZONE_RADIUS {
                let new_file = king_file + file_offset;
                let new_rank = king_rank + rank_offset;

                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    zone = BitBoard(zone.0 | (1u64 << (new_rank * 8 + new_file) as u64));
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        zones[i] = zone;
        i += 1;
    }
    zones
};

#[inline(always)]
fn evaluate_rooks(board: &Board, colour: Color) -> f32 {
    let rooks = board.pieces(Piece::Rook) & board.color_combined(colour);
    if rooks == EMPTY {
        return 0.0;
    }

    let our_pawns = board.pieces(Piece::Pawn) & board.color_combined(colour);
    let their_pawns = board.pieces(Piece::Pawn) & board.color_combined(!colour);

    let mut score = 0.0;
    for sq in rooks {
        let file_mask = get_file(sq.get_file());

        let our_file_pawns = (our_pawns & file_mask).popcnt();
        let their_file_pawns = (their_pawns & file_mask).popcnt();

        score += match (our_file_pawns == 0, their_file_pawns == 0) {
            (true, true) => ROOK_OPEN_FILE_BONUS,
            (true, false) => ROOK_SEMI_OPEN_FILE_BONUS,
            _ => 0.0,
        };

        // rook on seventh (second for Black)
        let rank = sq.get_rank();
        if (colour == Color::White && rank == Rank::Seventh)
            || (colour == Color::Black && rank == Rank::Second)
        {
            score += ROOK_ON_SEVENTH_BONUS;
        }
    }
    score
}

fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.popcnt() + bishops.popcnt() + 2 * rooks.popcnt() + 4 * queens.popcnt();

    (score.min(24) as f32) / 24.0
}
