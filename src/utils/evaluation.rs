use chess::{BitBoard, Board, BoardStatus, Color, MoveGen, Piece};

// Return final evaluation (positive = good for White, negative = good for Black)
pub fn evaluate_board(board: &Board) -> f32 {
    match board.status() {
        BoardStatus::Checkmate => {
            // If it’s White to move and board is checkmated => White lost
            if board.side_to_move() == chess::Color::White {
                return -10_000.0;
            } else {
                return 10_000.0;
            }
        }
        BoardStatus::Stalemate => return 0.0,
        BoardStatus::Ongoing => {}
    }

    let mut score = 0.0;
    score += evaluate_material(board, Color::White);
    score -= evaluate_material(board, Color::Black);

    score += count_mobility(board, Color::White) as f32;
    score -= count_mobility(board, Color::Black) as f32;

    score += evaluate_pawn_structure(board, Color::White);
    score -= evaluate_pawn_structure(board, Color::Black);

    score
}

fn evaluate_material(board: &Board, color: Color) -> f32 {
    let color_mask = board.color_combined(color);

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

    let pst = get_pst_refs(color);
    let mut psq_value = 0.0;
    if num_pawns > 0 {
        psq_value += sum_psq(pawn_mask, pst.pawn);
    }
    if num_knights > 0 {
        psq_value += sum_psq(knight_mask, pst.knight);
    }
    if num_bishops > 0 {
        psq_value += sum_psq(bishop_mask, pst.bishop);
    }
    if num_rooks > 0 {
        psq_value += sum_psq(rook_mask, pst.rook);
    }
    if num_queens > 0 {
        psq_value += sum_psq(queen_mask, pst.queen);
    }
    psq_value += sum_psq(king_mask, pst.king);

    // bonus for bishop pair
    let bishop_pair_bonus = if num_bishops >= 2 {
        BISHOP_PAIR_BONUS
    } else {
        0.0
    };

    return piece_value + psq_value + bishop_pair_bonus;
}

#[inline(always)]
fn sum_psq(bitboard: BitBoard, table: &[f32; 64]) -> f32 {
    let mut total = 0.0;
    for sq in bitboard {
        total += table[sq.to_index()];
    }
    total
}

const PAWN_VALUE: f32 = 100.0;
const KNIGHT_VALUE: f32 = 320.0;
const BISHOP_VALUE: f32 = 330.0;
const ROOK_VALUE: f32 = 500.0;
const QUEEN_VALUE: f32 = 900.0;
const BISHOP_PAIR_BONUS: f32 = 50.0;

struct PSTRefs<'a> {
    pawn: &'a [f32; 64],
    knight: &'a [f32; 64],
    bishop: &'a [f32; 64],
    rook: &'a [f32; 64],
    queen: &'a [f32; 64],
    king: &'a [f32; 64],
}

fn get_pst_refs(color: Color) -> PSTRefs<'static> {
    match color {
        Color::White => PSTRefs {
            pawn: &WHITE_PAWN_PST,
            knight: &WHITE_KNIGHT_PST,
            bishop: &WHITE_BISHOP_PST,
            rook: &WHITE_ROOK_PST,
            queen: &WHITE_QUEEN_PST,
            king: &WHITE_KING_PST,
        },
        Color::Black => PSTRefs {
            pawn: &BLACK_PAWN_PST,
            knight: &BLACK_KNIGHT_PST,
            bishop: &BLACK_BISHOP_PST,
            rook: &BLACK_ROOK_PST,
            queen: &BLACK_QUEEN_PST,
            king: &BLACK_KING_PST,
        },
    }
}

const WHITE_PAWN_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 1.0, 1.0, 2.0,
    3.0, 3.0, 2.0, 1.0, 1.0, 0.5, 0.5, 1.0, 2.5, 2.5, 1.0, 0.5, 0.5, 0.0, 0.0, 0.0, 2.0, 2.0, 0.0,
    0.0, 0.0, 0.5, -0.5, -1.0, 0.0, 0.0, -1.0, -0.5, 0.5, 0.5, 1.0, 1.0, -2.0, -2.0, 1.0, 1.0, 0.5,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

const BLACK_PAWN_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_PAWN_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_KNIGHT_PST: [f32; 64] = [
    -5.0, -14.0, -2.0, -2.0, -2.0, -2.0, -14.0, -5.0, -4.0, -2.0, 0.0, 0.5, 0.5, 0.0, -2.0, -4.0,
    -2.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -2.0, -2.0, 0.0, 1.0, 2.0, 2.0, 1.0, 0.0, -2.0, -2.0, 0.0,
    1.0, 2.0, 2.0, 1.0, 0.0, -2.0, -2.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -2.0, -4.0, -2.0, 0.0, 0.5,
    0.5, 0.0, -2.0, -4.0, -5.0, -4.0, -2.0, -2.0, -2.0, -2.0, -4.0, -5.0,
];

const BLACK_KNIGHT_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_KNIGHT_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_BISHOP_PST: [f32; 64] = [
    -2.0, -1.0, -11.0, -1.0, -1.0, -11.0, -1.0, -2.0, -1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.5, -1.0,
    -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.5,
    0.5, 1.0, 1.0, 0.5, 0.5, -1.0, -1.0, 0.0, 0.5, 1.0, 1.0, 0.5, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -1.0, -2.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -2.0,
];
const BLACK_BISHOP_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_BISHOP_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_ROOK_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, -0.5, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0,
];

const BLACK_ROOK_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_ROOK_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_QUEEN_PST: [f32; 64] = [
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0,
    0.5, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5,
    0.5, 0.5, 0.5, 0.0, -0.5, -1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -1.0, -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0,
];

const BLACK_QUEEN_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_QUEEN_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_KING_PST: [f32; 64] = [
    -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0, -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0,
    -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0, -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0,
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, -1.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0, -1.0,
    2.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 3.0, 1.0, 0.0, 0.0, 1.0, 3.0, 2.0,
];

const BLACK_KING_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_KING_PST[63 - i];
        i += 1;
    }
    table
};

fn count_mobility(board: &Board, color: Color) -> usize {
    if board.side_to_move() == color {
        // It's already our turn
        MoveGen::new_legal(board).count()
    } else {
        // Force a null move if possible
        if let Some(board_after_null) = board.null_move() {
            MoveGen::new_legal(&board_after_null).count()
        } else {
            // If null move is not available (e.g. in check),
            // fallback to 0 for now.
            0
        }
    }
}

fn evaluate_pawn_structure(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let pawns = board.pieces(Piece::Pawn) & board.color_combined(color);

    // File occupancy
    let mut files = [0; 8];
    for sq in pawns {
        let file = sq.get_file() as usize;
        files[file] += 1;
    }

    // check isolated pawns and doubled+ pawns
    for file in 0..8 {
        match files[file] {
            0 => continue,
            1 => {
                // Check for isolated pawns in the same loop
                let isolated = if file == 0 {
                    files[file + 1] == 0
                } else if file == 7 {
                    files[file - 1] == 0
                } else {
                    files[file - 1] == 0 && files[file + 1] == 0
                };
                if isolated {
                    score -= 30.0;
                }
            }
            2 => score -= 20.0, // doubled pawns
            _ => score -= 40.0, // tripled or more
        }
    }

    score
}
