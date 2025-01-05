use chess::{BitBoard, Color};

pub struct PSTRefs<'a> {
    pub pawn: &'a [f32; 64],
    pub knight: &'a [f32; 64],
    pub bishop: &'a [f32; 64],
    pub rook: &'a [f32; 64],
    pub queen: &'a [f32; 64],
    pub king: &'a [f32; 64],
}

#[inline(always)]
pub fn sum_pst(bitboard: BitBoard, table: &[f32; 64]) -> f32 {
    let mut total = 0.0;
    for sq in bitboard {
        total += table[sq.to_index()];
    }
    total
}

pub fn get_pst(color: Color) -> PSTRefs<'static> {
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

const fn invert_pst(source: &[f32; 64]) -> [f32; 64] {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -source[63 - i];
        i += 1;
    }
    table
}

// - Encourages pushing pawns to ranks 3-5
pub const WHITE_PAWN_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 1: a1..h1
    5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, // RANK 2
    1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 1.0, 1.0, // RANK 3
    0.5, 0.5, 1.0, 2.0, 2.0, 1.0, 0.5, 0.5, // RANK 4
    0.0, 0.0, 0.0, 1.5, 1.5, 0.0, 0.0, 0.0, // RANK 5
    0.5, -0.5, -1.0, 0.0, 0.0, -1.0, -0.5, 0.5, // RANK 6
    0.5, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 0.5, // RANK 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 8
];
const BLACK_PAWN_PST: [f32; 64] = invert_pst(&WHITE_PAWN_PST);

// - Mild center preference
// - Extra penalty on b1/g1 to encourage development
pub const WHITE_KNIGHT_PST: [f32; 64] = [
    -5.0, -6.0, -3.0, -3.0, -3.0, -3.0, -6.0, -5.0, // RANK 1: a1..h1
    -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, -2.0, -4.0, // RANK 2
    -3.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -3.0, // RANK 3
    -3.0, 0.0, 1.0, 1.5, 1.5, 1.0, 0.0, -3.0, // RANK 4
    -3.0, 0.0, 1.0, 1.5, 1.5, 1.0, 0.0, -3.0, // RANK 5
    -3.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -3.0, // RANK 6
    -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, -2.0, -4.0, // RANK 7
    -5.0, -4.0, -3.0, -3.0, -3.0, -3.0, -4.0, -5.0, // RANK 8
];
const BLACK_KNIGHT_PST: [f32; 64] = invert_pst(&WHITE_KNIGHT_PST);

// - Symmetrical center bonuses
// - Extra penalty on c1/f1 to encourage quicker movement
pub const WHITE_BISHOP_PST: [f32; 64] = [
    -2.0, -1.0, -3.0, -1.0, -1.0, -3.0, -1.0, -2.0, // RANK 1: a1..h1
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 2
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 3
    -1.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, -1.0, // RANK 4
    -1.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, -1.0, // RANK 5
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 6
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 7
    -2.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -2.0, // RANK 8
];
const BLACK_BISHOP_PST: [f32; 64] = invert_pst(&WHITE_BISHOP_PST);

// - Slight preference for open ranks/files
// - Extra 0.05 penalty on a1/h1 to encourage development
pub const WHITE_ROOK_PST: [f32; 64] = [
    -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, // RANK 1: a1..h
    0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, // RANK 2
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 3
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 4
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 5
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 6
    -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, // RANK 7
    0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, // RANK 8
];
const BLACK_ROOK_PST: [f32; 64] = invert_pst(&WHITE_ROOK_PST);

// - Mild center bonus
pub const WHITE_QUEEN_PST: [f32; 64] = [
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, // RANK 1: a1..h
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 2
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 3
    -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, // RANK 4
    -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, // RANK 5
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 6
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 7
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, // RANK 8
];
const BLACK_QUEEN_PST: [f32; 64] = invert_pst(&WHITE_QUEEN_PST);

// - Encourages King to stay safer on ranks 1-2 early
// - Mildly rewards stepping up in later phases
// - Encourage castling
pub const WHITE_KING_PST: [f32; 64] = [
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, // RANK 1: a1..h
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, // RANK 2
    -2.0, -3.0, -3.0, -3.0, -3.0, -3.0, -3.0, -2.0, // RANK 3
    -1.0, -2.0, -2.0, -3.0, -3.0, -2.0, -2.0, -1.0, // RANK 4
    0.0, -1.0, -1.0, -2.0, -2.0, -1.0, -1.0, 0.0, // RANK 5
    0.5, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 0.5, // RANK 6
    1.0, 1.0, 0.5, 0.0, 0.0, 0.5, 1.0, 1.0, // RANK 7
    2.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 2.0, // RANK 8
];
const BLACK_KING_PST: [f32; 64] = invert_pst(&WHITE_KING_PST);
