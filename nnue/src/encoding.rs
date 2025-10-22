use chess::{BitBoard, Board, Color, Piece, ALL_SQUARES, NUM_COLORS, NUM_PIECES, NUM_SQUARES};

// Board encoding feature counts
const NUM_PIECE_PLACEMENT_FEATURES: usize = NUM_SQUARES * NUM_PIECES * NUM_COLORS; // 768
const NUM_ATTACK_FEATURES: usize = NUM_SQUARES * 2; // 128 (white + black)
const NUM_SUPPORT_FEATURES: usize = NUM_SQUARES * 2; // 128 (white + black)
const NUM_SIDE_TO_MOVE_FEATURES: usize = 1;

pub const NUM_FEATURES: usize = NUM_PIECE_PLACEMENT_FEATURES
    + NUM_ATTACK_FEATURES
    + NUM_SUPPORT_FEATURES
    + NUM_SIDE_TO_MOVE_FEATURES;

pub const NUM_U64S: usize = NUM_FEATURES.div_ceil(64);

const PIECE_FEATURES_END: usize = NUM_PIECE_PLACEMENT_FEATURES;
const WHITE_ATTACKS_START: usize = PIECE_FEATURES_END;
const WHITE_ATTACKS_END: usize = WHITE_ATTACKS_START + NUM_SQUARES;
const BLACK_ATTACKS_START: usize = WHITE_ATTACKS_END;
const BLACK_ATTACKS_END: usize = BLACK_ATTACKS_START + NUM_SQUARES;
const WHITE_SUPPORT_START: usize = BLACK_ATTACKS_END;
const WHITE_SUPPORT_END: usize = WHITE_SUPPORT_START + NUM_SQUARES;
const BLACK_SUPPORT_START: usize = WHITE_SUPPORT_END;
const SIDE_TO_MOVE_IDX: usize = NUM_FEATURES - 1;

#[inline(always)]
pub fn encode_board(
    board: &Board,
    white_attacks: BitBoard,
    black_attacks: BitBoard,
    white_support: BitBoard,
    black_support: BitBoard,
) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    // Piece placements
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq.to_index() * 12 + piece_color_to_index(piece, color);
            features[offset] = 1.0;
        }
    }

    // White attacks
    for sq in white_attacks {
        features[WHITE_ATTACKS_START + sq.to_index()] = 1.0;
    }

    // Black attacks
    for sq in black_attacks {
        features[BLACK_ATTACKS_START + sq.to_index()] = 1.0;
    }

    // White support
    for sq in white_support {
        features[WHITE_SUPPORT_START + sq.to_index()] = 1.0;
    }

    // Black support
    for sq in black_support {
        features[BLACK_SUPPORT_START + sq.to_index()] = 1.0;
    }

    // Side to move
    if board.side_to_move() == Color::White {
        features[SIDE_TO_MOVE_IDX] = 1.0;
    }

    features
}

#[inline(always)]
pub fn encode_board_bitset(
    board: &Board,
    white_attacks: BitBoard,
    black_attacks: BitBoard,
    white_support: BitBoard,
    black_support: BitBoard,
) -> [u64; NUM_U64S] {
    let mut words = [0u64; NUM_U64S];

    // Piece placements
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq.to_index() * 12 + piece_color_to_index(piece, color);
            let word_idx = offset / 64;
            let bit_idx = offset % 64;
            words[word_idx] |= 1u64 << bit_idx;
        }
    }

    // White attacks
    for sq in white_attacks {
        let idx = WHITE_ATTACKS_START + sq.to_index();
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Black attacks
    for sq in black_attacks {
        let idx = BLACK_ATTACKS_START + sq.to_index();
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // White support
    for sq in white_support {
        let idx = WHITE_SUPPORT_START + sq.to_index();
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Black support
    for sq in black_support {
        let idx = BLACK_SUPPORT_START + sq.to_index();
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Side to move
    if board.side_to_move() == Color::White {
        let idx = SIDE_TO_MOVE_IDX;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    words
}

#[inline(always)]
fn piece_color_to_index(piece: Piece, color: Color) -> usize {
    match (color, piece) {
        (Color::White, Piece::Pawn) => 0,
        (Color::White, Piece::Knight) => 1,
        (Color::White, Piece::Bishop) => 2,
        (Color::White, Piece::Rook) => 3,
        (Color::White, Piece::Queen) => 4,
        (Color::White, Piece::King) => 5,

        (Color::Black, Piece::Pawn) => 6,
        (Color::Black, Piece::Knight) => 7,
        (Color::Black, Piece::Bishop) => 8,
        (Color::Black, Piece::Rook) => 9,
        (Color::Black, Piece::Queen) => 10,
        (Color::Black, Piece::King) => 11,
    }
}
