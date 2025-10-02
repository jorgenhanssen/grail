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
pub fn sum_pst(bitboard: BitBoard, pst: &PST, phase: f32, inv_phase: f32) -> i16 {
    let mut total = 0.0;
    for sq in bitboard {
        let idx = sq.to_index();
        total += pst.mg[idx] * phase + pst.eg[idx] * inv_phase;
    }
    total.round() as i16
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

// Pawns: Stronger push to ranks 4-6, eg rewards advancement more
pub const WHITE_PAWN_MG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 1 (a1-h1)
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 2
    10.0, 12.0, 15.0, 20.0, 20.0, 15.0, 12.0, 10.0, // rank 3
    15.0, 18.0, 25.0, 30.0, 30.0, 25.0, 18.0, 15.0, // rank 4
    20.0, 25.0, 35.0, 40.0, 40.0, 35.0, 25.0, 20.0, // rank 5
    10.0, 12.0, 15.0, 20.0, 20.0, 15.0, 12.0, 10.0, // rank 6
    -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, // rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 8
];
pub const WHITE_PAWN_EG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 2
    10.0, 12.0, 15.0, 20.0, 20.0, 15.0, 12.0, 10.0, // rank 3
    20.0, 25.0, 30.0, 35.0, 35.0, 30.0, 25.0, 20.0, // rank 4
    35.0, 40.0, 45.0, 50.0, 50.0, 45.0, 40.0, 35.0, // rank 5
    50.0, 55.0, 60.0, 70.0, 70.0, 60.0, 55.0, 50.0, // rank 6
    80.0, 85.0, 90.0, 100.0, 100.0, 90.0, 85.0, 80.0, // rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 8
];

// Knights: Boost central/outposts (e.g., d4/e5/f3), penalize edges/back ranks
pub const WHITE_KNIGHT_MG_PST: [f32; 64] = [
    -50.0, -40.0, -30.0, -30.0, -30.0, -30.0, -40.0, -50.0, // rank 1
    -40.0, -20.0, 0.0, 5.0, 5.0, 0.0, -20.0, -40.0, // rank 2
    -30.0, 5.0, 10.0, 15.0, 15.0, 10.0, 5.0, -30.0, // rank 3
    -30.0, 0.0, 15.0, 20.0, 20.0, 15.0, 0.0, -30.0, // rank 4
    -30.0, 5.0, 20.0, 25.0, 25.0, 20.0, 5.0, -30.0, // rank 5
    -30.0, 0.0, 15.0, 20.0, 20.0, 15.0, 0.0, -30.0, // rank 6
    -40.0, -20.0, 0.0, 0.0, 0.0, 0.0, -20.0, -40.0, // rank 7
    -50.0, -40.0, -30.0, -30.0, -30.0, -30.0, -40.0, -50.0, // rank 8
];
pub const WHITE_KNIGHT_EG_PST: [f32; 64] = [
    -50.0, -40.0, -30.0, -20.0, -20.0, -30.0, -40.0, -50.0, // rank 1
    -40.0, -20.0, 0.0, 0.0, 0.0, 0.0, -20.0, -40.0, // rank 2
    -30.0, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0, -30.0, // rank 3
    -20.0, 10.0, 20.0, 20.0, 20.0, 20.0, 10.0, -20.0, // rank 4
    -20.0, 10.0, 20.0, 20.0, 20.0, 20.0, 10.0, -20.0, // rank 5
    -30.0, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0, -30.0, // rank 6
    -40.0, -20.0, 0.0, 0.0, 0.0, 0.0, -20.0, -40.0, // rank 7
    -50.0, -40.0, -30.0, -20.0, -20.0, -30.0, -40.0, -50.0, // rank 8
];

// Bishops: Symmetrical center bonuses, penalize back ranks/edges
pub const WHITE_BISHOP_MG_PST: [f32; 64] = [
    -20.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -20.0, // rank 1
    -10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 5.0, -10.0, // rank 2
    -10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, -10.0, // rank 3
    -10.0, 0.0, 10.0, 15.0, 15.0, 10.0, 0.0, -10.0, // rank 4
    -10.0, 5.0, 15.0, 20.0, 20.0, 15.0, 5.0, -10.0, // rank 5
    -10.0, 0.0, 10.0, 15.0, 15.0, 10.0, 0.0, -10.0, // rank 6
    -10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -10.0, // rank 7
    -20.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -20.0, // rank 8
];
pub const WHITE_BISHOP_EG_PST: [f32; 64] = [
    -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, // rank 1
    -10.0, -5.0, 0.0, 0.0, 0.0, 0.0, -5.0, -10.0, // rank 2
    -5.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 3
    0.0, 5.0, 10.0, 15.0, 15.0, 10.0, 5.0, 0.0, // rank 4
    0.0, 5.0, 10.0, 15.0, 15.0, 10.0, 5.0, 0.0, // rank 5
    -5.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 6
    -10.0, -5.0, 0.0, 0.0, 0.0, 0.0, -5.0, -10.0, // rank 7
    -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, // rank 8
];

// Rooks: Preference for open ranks/files, slight center bonus
pub const WHITE_ROOK_MG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 5.0, 5.0, 0.0, 0.0, 0.0, // rank 1
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 2
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 3
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 4
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 5
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 6
    5.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 5.0, // rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 8
];
pub const WHITE_ROOK_EG_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 1
    5.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 5.0, // rank 2
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 3
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 4
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 5
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // rank 6
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 7
    -10.0, -10.0, -10.0, -5.0, -5.0, -10.0, -10.0, -10.0, // rank 8
];

// Queens: Mild center bonus, penalize back ranks
pub const WHITE_QUEEN_MG_PST: [f32; 64] = [
    -20.0, -10.0, -10.0, -5.0, -5.0, -10.0, -10.0, -20.0, // rank 1
    -10.0, 0.0, 5.0, 0.0, 0.0, 0.0, 0.0, -10.0, // rank 2
    -10.0, 5.0, 5.0, 5.0, 5.0, 5.0, 0.0, -10.0, // rank 3
    0.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 4
    -5.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 5
    -10.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -10.0, // rank 6
    -10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -10.0, // rank 7
    -20.0, -10.0, -10.0, -5.0, -5.0, -10.0, -10.0, -20.0, // rank 8
];
pub const WHITE_QUEEN_EG_PST: [f32; 64] = [
    -10.0, -10.0, -5.0, 0.0, 0.0, -5.0, -10.0, -10.0, // rank 1
    -5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -5.0, // rank 2
    -5.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 3
    0.0, 5.0, 10.0, 10.0, 10.0, 10.0, 5.0, 0.0, // rank 4
    0.0, 5.0, 10.0, 10.0, 10.0, 10.0, 5.0, 0.0, // rank 5
    -5.0, 0.0, 5.0, 5.0, 5.0, 5.0, 0.0, -5.0, // rank 6
    -5.0, 0.0, 0.0, 5.0, 5.0, 0.0, 0.0, -5.0, // rank 7
    -10.0, -10.0, -10.0, -5.0, -5.0, -10.0, -10.0, -10.0, // rank 8
];

// Kings: Encourage castling sides in mg, central in eg
pub const WHITE_KING_MG_PST: [f32; 64] = [
    20.0, 30.0, 10.0, 0.0, 0.0, 10.0, 30.0, 20.0, // rank 1
    20.0, 20.0, 0.0, 0.0, 0.0, 0.0, 20.0, 20.0, // rank 2
    10.0, 0.0, -10.0, -20.0, -20.0, -10.0, 0.0, 10.0, // rank 3
    0.0, 0.0, -20.0, -30.0, -30.0, -20.0, 0.0, 0.0, // rank 4
    -10.0, -20.0, -30.0, -40.0, -40.0, -30.0, -20.0, -10.0, // rank 5
    -20.0, -30.0, -40.0, -50.0, -50.0, -40.0, -30.0, -20.0, // rank 6
    -30.0, -40.0, -50.0, -60.0, -60.0, -50.0, -40.0, -30.0, // rank 7
    -40.0, -50.0, -60.0, -70.0, -70.0, -60.0, -50.0, -40.0, // rank 8
];
pub const WHITE_KING_EG_PST: [f32; 64] = [
    -50.0, -30.0, -20.0, -10.0, -10.0, -20.0, -30.0, -50.0, // rank 1
    -30.0, -20.0, 0.0, 5.0, 5.0, 0.0, -20.0, -30.0, // rank 2
    -20.0, 0.0, 10.0, 20.0, 20.0, 10.0, 0.0, -20.0, // rank 3
    -10.0, 5.0, 20.0, 30.0, 30.0, 20.0, 5.0, -10.0, // rank 4
    -10.0, 5.0, 20.0, 30.0, 30.0, 20.0, 5.0, -10.0, // rank 5
    -20.0, 0.0, 10.0, 20.0, 20.0, 10.0, 0.0, -20.0, // rank 6
    -30.0, -20.0, 0.0, 5.0, 5.0, 0.0, -20.0, -30.0, // rank 7
    -50.0, -30.0, -20.0, -10.0, -10.0, -20.0, -30.0, -50.0, // rank 8
];

const BLACK_PAWN_MG_PST: [f32; 64] = invert_pst(&WHITE_PAWN_MG_PST);
const BLACK_PAWN_EG_PST: [f32; 64] = invert_pst(&WHITE_PAWN_EG_PST);
const BLACK_KNIGHT_MG_PST: [f32; 64] = invert_pst(&WHITE_KNIGHT_MG_PST);
const BLACK_KNIGHT_EG_PST: [f32; 64] = invert_pst(&WHITE_KNIGHT_EG_PST);
const BLACK_BISHOP_MG_PST: [f32; 64] = invert_pst(&WHITE_BISHOP_MG_PST);
const BLACK_BISHOP_EG_PST: [f32; 64] = invert_pst(&WHITE_BISHOP_EG_PST);
const BLACK_ROOK_MG_PST: [f32; 64] = invert_pst(&WHITE_ROOK_MG_PST);
const BLACK_ROOK_EG_PST: [f32; 64] = invert_pst(&WHITE_ROOK_EG_PST);
const BLACK_QUEEN_MG_PST: [f32; 64] = invert_pst(&WHITE_QUEEN_MG_PST);
const BLACK_QUEEN_EG_PST: [f32; 64] = invert_pst(&WHITE_QUEEN_EG_PST);
const BLACK_KING_MG_PST: [f32; 64] = invert_pst(&WHITE_KING_MG_PST);
const BLACK_KING_EG_PST: [f32; 64] = invert_pst(&WHITE_KING_EG_PST);
