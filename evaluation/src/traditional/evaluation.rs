use chess::{
    get_adjacent_files, get_file, BitBoard, Board, BoardStatus, Color, Piece, Rank, ALL_FILES,
    EMPTY,
};

use crate::scores::MATE_VALUE;
use crate::traditional::pst::{get_pst, sum_pst};
use crate::traditional::values::{
    piece_value, BISHOP_VALUE, KNIGHT_VALUE, PASSED_PAWN_BONUS, PAWN_VALUE, QUEEN_VALUE,
    ROOK_ON_SEVENTH_BONUS, ROOK_OPEN_FILE_BONUS, ROOK_SEMI_OPEN_FILE_BONUS, ROOK_VALUE,
};

// Return final evaluation (positive = good for White, negative = good for Black)
pub fn evaluate_board(board: &Board) -> i16 {
    let is_white = board.side_to_move() == Color::White;

    match board.status() {
        BoardStatus::Checkmate => {
            // If it's White to move and board is checkmated => White lost
            if is_white {
                return -MATE_VALUE;
            } else {
                return MATE_VALUE;
            }
        }
        BoardStatus::Stalemate => return 0,
        BoardStatus::Ongoing => {}
    }

    let phase = game_phase(board);

    let white_mask = board.color_combined(Color::White);
    let black_mask = board.color_combined(Color::Black);

    let mut cp: i16 = 0;
    cp += evaluate_material(board, Color::White, &white_mask, phase);
    cp -= evaluate_material(board, Color::Black, &black_mask, phase);

    cp += evaluate_pawn_structure(board, Color::White);
    cp -= evaluate_pawn_structure(board, Color::Black);

    cp += evaluate_rooks(board, Color::White);
    cp -= evaluate_rooks(board, Color::Black);

    cp += evaluate_king_safety(board, Color::White);
    cp -= evaluate_king_safety(board, Color::Black);

    cp
}

#[inline(always)]
fn evaluate_material(board: &Board, color: Color, color_mask: &BitBoard, phase: f32) -> i16 {
    let pawn_mask = board.pieces(Piece::Pawn) & color_mask;
    let knight_mask = board.pieces(Piece::Knight) & color_mask;
    let bishop_mask = board.pieces(Piece::Bishop) & color_mask;
    let rook_mask = board.pieces(Piece::Rook) & color_mask;
    let queen_mask = board.pieces(Piece::Queen) & color_mask;
    let king_mask = board.pieces(Piece::King) & color_mask;

    let mut cp = 0i16;

    cp += PAWN_VALUE * pawn_mask.popcnt() as i16;
    cp += KNIGHT_VALUE * knight_mask.popcnt() as i16;
    cp += BISHOP_VALUE * bishop_mask.popcnt() as i16;
    cp += ROOK_VALUE * rook_mask.popcnt() as i16;
    cp += QUEEN_VALUE * queen_mask.popcnt() as i16;

    let pst = get_pst(color);
    if pawn_mask != EMPTY {
        cp += sum_pst(pawn_mask, pst.pawn, phase);
    }
    if knight_mask != EMPTY {
        cp += sum_pst(knight_mask, pst.knight, phase);
    }
    if bishop_mask != EMPTY {
        cp += sum_pst(bishop_mask, pst.bishop, phase);
    }
    if rook_mask != EMPTY {
        cp += sum_pst(rook_mask, pst.rook, phase);
    }
    if queen_mask != EMPTY {
        cp += sum_pst(queen_mask, pst.queen, phase);
    }
    cp += sum_pst(king_mask, pst.king, phase); // king always present

    // bonus for bishop pair
    if bishop_mask.popcnt() >= 2 {
        cp += 50;
    }

    cp
}

#[inline(always)]
fn evaluate_pawn_structure(board: &Board, color: Color) -> i16 {
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

#[inline(always)]
fn evaluate_king_safety(board: &Board, color: Color) -> i16 {
    let king_square = board.king_square(color);
    let king_zone = KING_ZONES[king_square.to_index()];
    let enemy_color = !color;
    let enemy_pieces = board.color_combined(enemy_color);

    let queens = (enemy_pieces & board.pieces(Piece::Queen) & king_zone).popcnt() as i16;
    let rooks = (enemy_pieces & board.pieces(Piece::Rook) & king_zone).popcnt() as i16;
    let bishops = (enemy_pieces & board.pieces(Piece::Bishop) & king_zone).popcnt() as i16;
    let knights = (enemy_pieces & board.pieces(Piece::Knight) & king_zone).popcnt() as i16;
    let pawns = (enemy_pieces & board.pieces(Piece::Pawn) & king_zone).popcnt() as i16;

    let mut cp = 0i16;
    cp -= queens * piece_value(Piece::Queen);
    cp -= rooks * piece_value(Piece::Rook);
    cp -= bishops * piece_value(Piece::Bishop);
    cp -= knights * piece_value(Piece::Knight);
    cp -= pawns * piece_value(Piece::Pawn);

    // Let's do 30% of the value of the pieces
    (0.3 * (cp as f32)).round() as i16
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
fn evaluate_rooks(board: &Board, color: Color) -> i16 {
    let rooks = board.pieces(Piece::Rook) & board.color_combined(color);
    if rooks == EMPTY {
        return 0;
    }

    let our_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    let their_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);

    let mut cp = 0i16;
    for sq in rooks {
        let file_mask = get_file(sq.get_file());

        let our_file_pawns = (our_pawns & file_mask).popcnt();
        let their_file_pawns = (their_pawns & file_mask).popcnt();

        cp += match (our_file_pawns == 0, their_file_pawns == 0) {
            (true, true) => ROOK_OPEN_FILE_BONUS,
            (true, false) => ROOK_SEMI_OPEN_FILE_BONUS,
            _ => 0,
        };

        // rook on seventh (second for Black)
        let rank = sq.get_rank();
        if (color == Color::White && rank == Rank::Seventh)
            || (color == Color::Black && rank == Rank::Second)
        {
            cp += ROOK_ON_SEVENTH_BONUS;
        }
    }
    cp
}

fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.popcnt() + bishops.popcnt() + 2 * rooks.popcnt() + 4 * queens.popcnt();

    (score.min(24) as f32) / 24.0
}
