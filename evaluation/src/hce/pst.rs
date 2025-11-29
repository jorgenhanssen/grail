use cozy_chess::{BitBoard, Color};

/// Piece-Square Table: position-dependent bonuses/penalties for each piece.
/// Separate tables for middlegame (mg) and endgame (eg), interpolated by game phase.
///
/// Values are in centipawns. Positive = good square, negative = bad square.
/// Tables are defined for White (a1=index 0, h8=index 63), Black tables are mirrored.
///
/// <https://www.chessprogramming.org/Piece-Square_Tables>
#[allow(clippy::upper_case_acronyms)]
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

pub fn sum_pst(bitboard: BitBoard, pst: &PST, phase: f32, inv_phase: f32) -> i16 {
    let mut total = 0.0;
    for sq in bitboard {
        let idx = sq as usize;
        total += pst.mg[idx] * phase + pst.eg[idx] * inv_phase;
    }
    total.round() as i16
}

pub fn get_pst(color: Color) -> &'static PSTRefs<'static> {
    match color {
        Color::White => &PST_TABLE[0],
        Color::Black => &PST_TABLE[1],
    }
}

/// Mirrors a White PST vertically to create Black's perspective.
/// Black's a8 corresponds to White's a1, etc.
const fn invert_pst(source: &[f32; 64]) -> [f32; 64] {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        let rank = i / 8;
        let file = i % 8;
        let flipped_index = (7 - rank) * 8 + file;
        table[i] = source[flipped_index];
        i += 1;
    }
    table
}

// Pawns MG: Reward central control (d4/e4), slight penalty for premature advancement to 7th.
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

// Pawns EG: Strong bonus for advanced pawns. Passed pawns on 6th/7th are very valuable.
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

// Knights MG: Outposts in enemy territory are ideal. Center controls 8 squares vs 2-4 on edges.
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

// Knights EG: Still centralized but less critical. "Knight on the rim is dim" still applies.
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

// Bishops MG: Long diagonals maximize scope. Fianchetto (b2/g2) controls key squares.
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

// Bishops EG: Central placement for maximum reach. Edges and corners limit mobility.
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

// Rooks MG: Central files (d/e) slightly preferred. 7th rank is very strong (pins pawns).
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

// Rooks EG: Active rooks are crucial; back rank can become a liability.
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

// Queens MG: Discourage early queen development—vulnerable to tempo-gaining attacks.
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

// Queens EG: Central queen is powerful; corners and edges reduce mobility.
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

// Kings MG: Safety first—castled positions (g1/c1) are safest, center is exposed.
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

// Kings EG: King becomes active; centralization is key for endgame technique.
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

// Black PSTs are just vertically mirrored White PSTs
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
