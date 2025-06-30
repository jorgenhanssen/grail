use chess::{BitBoard, Color};

#[derive(Clone, Copy)]
pub struct PST<'a> {
    pub mg: &'a [f32; 64],
    pub eg: &'a [f32; 64],
}

pub struct PSTRefs<'a> {
    pub pawn: &'a PST<'a>,
    pub knight: &'a PST<'a>,
    pub bishop: &'a PST<'a>,
    pub rook: &'a PST<'a>,
    pub queen: &'a PST<'a>,
    pub king: &'a PST<'a>,
}

const PST_TABLE: [PSTRefs; 2] = [
    // index 0 → White
    PSTRefs {
        pawn: &PST {
            mg: &WHITE_PAWN_MG_PST,
            eg: &WHITE_PAWN_EG_PST,
        },
        knight: &PST {
            mg: &WHITE_KNIGHT_MG_PST,
            eg: &WHITE_KNIGHT_EG_PST,
        },
        bishop: &PST {
            mg: &WHITE_BISHOP_MG_PST,
            eg: &WHITE_BISHOP_EG_PST,
        },
        rook: &PST {
            mg: &WHITE_ROOK_MG_PST,
            eg: &WHITE_ROOK_EG_PST,
        },
        queen: &PST {
            mg: &WHITE_QUEEN_MG_PST,
            eg: &WHITE_QUEEN_EG_PST,
        },
        king: &PST {
            mg: &WHITE_KING_MG_PST,
            eg: &WHITE_KING_EG_PST,
        },
    },
    // index 1 → Black
    PSTRefs {
        pawn: &PST {
            mg: &BLACK_PAWN_MG_PST,
            eg: &BLACK_PAWN_EG_PST,
        },
        knight: &PST {
            mg: &BLACK_KNIGHT_MG_PST,
            eg: &BLACK_KNIGHT_EG_PST,
        },
        bishop: &PST {
            mg: &BLACK_BISHOP_MG_PST,
            eg: &BLACK_BISHOP_EG_PST,
        },
        rook: &PST {
            mg: &BLACK_ROOK_MG_PST,
            eg: &BLACK_ROOK_EG_PST,
        },
        queen: &PST {
            mg: &BLACK_QUEEN_MG_PST,
            eg: &BLACK_QUEEN_EG_PST,
        },
        king: &PST {
            mg: &BLACK_KING_MG_PST,
            eg: &BLACK_KING_EG_PST,
        },
    },
];

#[inline(always)]
pub fn sum_pst(bitboard: BitBoard, pst: &PST, phase: f32) -> f32 {
    let mut total = 0.0;
    for sq in bitboard {
        let mg_blend = pst.mg[sq.to_index()] * phase;
        let eg_blend = pst.eg[sq.to_index()] * (1.0 - phase);

        total += mg_blend + eg_blend;
    }
    total * 2.0
}

#[inline(always)]
pub fn get_pst(color: Color) -> &'static PSTRefs<'static> {
    match color {
        Color::White => &PST_TABLE[0],
        Color::Black => &PST_TABLE[1],
    }
}

const fn invert_pst(source: &[f32; 64]) -> [f32; 64] {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        let rank = i / 8;
        let file = i % 8;
        let flipped_index = (7 - rank) * 8 + file; // vertical mirror
        table[i] = source[flipped_index];
        i += 1;
    }
    table
}

// - Encourages pushing pawns to ranks 3-5
pub const WHITE_PAWN_MG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 2
    0.10, 0.11, 0.12, 0.14, 0.14, 0.12, 0.11, 0.10, // rank 3
    0.18, 0.19, 0.21, 0.23, 0.23, 0.21, 0.19, 0.18, // rank 4
    0.26, 0.27, 0.29, 0.32, 0.32, 0.29, 0.27, 0.26, // rank 5
    0.10, 0.11, 0.12, 0.14, 0.14, 0.12, 0.11, 0.10, // rank 6
    -0.06, -0.06, -0.06, -0.06, -0.06, -0.06, -0.06, -0.06, // rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 8
];
pub const WHITE_PAWN_EG_PST: [f32; 64] = [
    // a1…h1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 2
    0.12, 0.13, 0.14, 0.16, 0.16, 0.14, 0.13, 0.12, // rank 3
    0.24, 0.26, 0.28, 0.30, 0.30, 0.28, 0.26, 0.24, // rank 4
    0.38, 0.40, 0.42, 0.45, 0.45, 0.42, 0.40, 0.38, // rank 5
    0.54, 0.56, 0.58, 0.62, 0.62, 0.58, 0.56, 0.54, // rank 6
    0.70, 0.72, 0.75, 0.80, 0.80, 0.75, 0.72, 0.70, // rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 8 (promotion)
];
const BLACK_PAWN_MG_PST: [f32; 64] = invert_pst(&WHITE_PAWN_MG_PST);
const BLACK_PAWN_EG_PST: [f32; 64] = invert_pst(&WHITE_PAWN_EG_PST);

// - Mild center preference
// - Extra penalty on b1/g1 to encourage development
pub const WHITE_KNIGHT_MG_PST: [f32; 64] = [
    -5.0, -6.0, -3.0, -3.0, -3.0, -3.0, -6.0, -5.0, // RANK 1: a1..h1
    -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, -2.0, -4.0, // RANK 2
    -3.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -3.0, // RANK 3
    -3.0, 0.0, 1.0, 1.5, 1.5, 1.0, 0.0, -3.0, // RANK 4
    -3.0, 0.0, 1.0, 1.5, 1.5, 1.0, 0.0, -3.0, // RANK 5
    -3.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -3.0, // RANK 6
    -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, -2.0, -4.0, // RANK 7
    -5.0, -4.0, -3.0, -3.0, -3.0, -3.0, -4.0, -5.0, // RANK 8
];
pub const WHITE_KNIGHT_EG_PST: [f32; 64] = [
    -4.0, -4.0, -2.0, -2.0, -2.0, -2.0, -4.0, -4.0, -2.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -2.0,
    -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 1.0, -1.0, -1.0, 1.0,
    2.0, 2.0, 2.0, 2.0, 1.0, -1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0,
    0.0, 0.0, -1.0, -2.0, -4.0, -4.0, -2.0, -2.0, -2.0, -2.0, -4.0, -4.0,
];
const BLACK_KNIGHT_MG_PST: [f32; 64] = invert_pst(&WHITE_KNIGHT_MG_PST);
const BLACK_KNIGHT_EG_PST: [f32; 64] = invert_pst(&WHITE_KNIGHT_EG_PST);

// - Symmetrical center bonuses
// - Extra penalty on c1/f1 to encourage quicker movement
pub const WHITE_BISHOP_MG_PST: [f32; 64] = [
    -2.0, -1.0, -3.0, -1.0, -1.0, -3.0, -1.0, -2.0, // RANK 1: a1..h1
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 2
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 3
    -1.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, -1.0, // RANK 4
    -1.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, -1.0, // RANK 5
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 6
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 7
    -2.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -2.0, // RANK 8
];
pub const WHITE_BISHOP_EG_PST: [f32; 64] = [
    -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, // RANK 1
    -0.5, -0.25, -0.25, -0.25, -0.25, -0.25, -0.25, -0.5, // RANK 2
    -0.25, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.25, // RANK 3
    0.0, 0.5, 1.0, 1.25, 1.25, 1.0, 0.5, 0.0, // RANK 4
    0.0, 0.5, 1.0, 1.25, 1.25, 1.0, 0.5, 0.0, // RANK 5
    -0.25, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.25, // RANK 6
    -0.5, -0.25, -0.25, -0.25, -0.25, -0.25, -0.25, -0.5, // RANK 7
    -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, // RANK 8
];
const BLACK_BISHOP_MG_PST: [f32; 64] = invert_pst(&WHITE_BISHOP_MG_PST);
const BLACK_BISHOP_EG_PST: [f32; 64] = invert_pst(&WHITE_BISHOP_EG_PST);

// - Slight preference for open ranks/files
// - Extra 0.05 penalty on a1/h1 to encourage development
pub const WHITE_ROOK_MG_PST: [f32; 64] = [
    -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, // RANK 1: a1..h
    0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, // RANK 2
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 3
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 4
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 5
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // RANK 6
    -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5, // RANK 7
    0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, // RANK 8
];
pub const WHITE_ROOK_EG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.25, 0.25, 0.0, 0.0, 0.0, // RANK 1
    0.25, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.25, // RANK 2
    0.25, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.25, // RANK 3
    0.25, 0.5, 0.5, 0.75, 0.75, 0.5, 0.5, 0.25, // RANK 4
    0.25, 0.5, 0.5, 0.75, 0.75, 0.5, 0.5, 0.25, // RANK 5
    0.25, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.25, // RANK 6
    0.25, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.25, // RANK 7
    0.0, 0.0, 0.0, 0.25, 0.25, 0.0, 0.0, 0.0, // RANK 8
];
const BLACK_ROOK_MG_PST: [f32; 64] = invert_pst(&WHITE_ROOK_MG_PST);
const BLACK_ROOK_EG_PST: [f32; 64] = invert_pst(&WHITE_ROOK_EG_PST);

// - Mild center bonus
pub const WHITE_QUEEN_MG_PST: [f32; 64] = [
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, // RANK 1: a1..h
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 2
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 3
    -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, // RANK 4
    -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, // RANK 5
    -1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, // RANK 6
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, // RANK 7
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, // RANK 8
];
pub const WHITE_QUEEN_EG_PST: [f32; 64] = [
    -1.0, -1.0, -0.5, -0.25, -0.25, -0.5, -1.0, -1.0, // RANK 1
    -0.5, -0.25, 0.0, 0.0, 0.0, 0.0, -0.25, -0.5, // RANK 2
    -0.25, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.25, // RANK 3
    0.0, 0.5, 0.75, 1.0, 1.0, 0.75, 0.5, 0.0, // RANK 4
    0.0, 0.5, 0.75, 1.0, 1.0, 0.75, 0.5, 0.0, // RANK 5
    -0.25, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.25, // RANK 6
    -0.5, -0.25, 0.0, 0.0, 0.0, 0.0, -0.25, -0.5, // RANK 7
    -1.0, -1.0, -0.5, -0.25, -0.25, -0.5, -1.0, -1.0, // RANK 8
];
const BLACK_QUEEN_MG_PST: [f32; 64] = invert_pst(&WHITE_QUEEN_MG_PST);
const BLACK_QUEEN_EG_PST: [f32; 64] = invert_pst(&WHITE_QUEEN_EG_PST);

// - Encourages King to stay safer on ranks 1-2 early
// - Mildly rewards stepping up in later phases
// - Encourage castling
pub const WHITE_KING_MG_PST: [f32; 64] = [
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, // RANK 1: a1..h
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, // RANK 2
    -2.0, -3.0, -3.0, -3.0, -3.0, -3.0, -3.0, -2.0, // RANK 3
    -1.0, -2.0, -2.0, -3.0, -3.0, -2.0, -2.0, -1.0, // RANK 4
    0.0, -1.0, -1.0, -2.0, -2.0, -1.0, -1.0, 0.0, // RANK 5
    0.5, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 0.5, // RANK 6
    1.0, 1.0, 0.5, 0.0, 0.0, 0.5, 1.0, 1.0, // RANK 7
    2.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 2.0, // RANK 8
];
pub const WHITE_KING_EG_PST: [f32; 64] = [
    -4.0, -3.0, -2.0, -1.0, -1.0, -2.0, -3.0, -4.0, // RANK 1
    -3.0, -1.5, -1.0, 0.0, 0.0, -1.0, -1.5, -3.0, // RANK 2
    -2.0, -1.0, 1.0, 1.5, 1.5, 1.0, -1.0, -2.0, // RANK 3
    -1.0, 0.0, 1.5, 2.5, 2.5, 1.5, 0.0, -1.0, // RANK 4
    -1.0, 0.0, 1.5, 2.5, 2.5, 1.5, 0.0, -1.0, // RANK 5
    -2.0, -1.0, 1.0, 1.5, 1.5, 1.0, -1.0, -2.0, // RANK 6
    -3.0, -1.5, -1.0, 0.0, 0.0, -1.0, -1.5, -3.0, // RANK 7
    -4.0, -3.0, -2.0, -1.0, -1.0, -2.0, -3.0, -4.0, // RANK 8
];
const BLACK_KING_MG_PST: [f32; 64] = invert_pst(&WHITE_KING_MG_PST);
const BLACK_KING_EG_PST: [f32; 64] = invert_pst(&WHITE_KING_EG_PST);
