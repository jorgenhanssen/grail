use cozy_chess::{BitBoard, Board, Color, Piece, Square};

// Board encoding feature counts
const NUM_PIECE_PLACEMENT_FEATURES: usize = Square::NUM * Piece::NUM * Color::NUM; // 768
const NUM_SUPPORT_FEATURES: usize = Square::NUM * 2; // 128 (white + black defended pieces)
const NUM_SPACE_FEATURES: usize = Square::NUM * 2; // 128 (white + black controlled non-piece squares)
const NUM_THREAT_FEATURES: usize = Square::NUM * 2; // 128 (white + black threatened pieces)
const NUM_SIDE_TO_MOVE_FEATURES: usize = 1;

pub const NUM_FEATURES: usize = NUM_PIECE_PLACEMENT_FEATURES
    + NUM_SUPPORT_FEATURES
    + NUM_SPACE_FEATURES
    + NUM_THREAT_FEATURES
    + NUM_SIDE_TO_MOVE_FEATURES; // 1153 total

pub const NUM_U64S: usize = NUM_FEATURES.div_ceil(64);

const PIECE_FEATURES_END: usize = NUM_PIECE_PLACEMENT_FEATURES;
const WHITE_SUPPORT_START: usize = PIECE_FEATURES_END;
const WHITE_SUPPORT_END: usize = WHITE_SUPPORT_START + Square::NUM;
const BLACK_SUPPORT_START: usize = WHITE_SUPPORT_END;
const BLACK_SUPPORT_END: usize = BLACK_SUPPORT_START + Square::NUM;
const WHITE_SPACE_START: usize = BLACK_SUPPORT_END;
const WHITE_SPACE_END: usize = WHITE_SPACE_START + Square::NUM;
const BLACK_SPACE_START: usize = WHITE_SPACE_END;
const BLACK_SPACE_END: usize = BLACK_SPACE_START + Square::NUM;
const WHITE_THREATS_START: usize = BLACK_SPACE_END;
const WHITE_THREATS_END: usize = WHITE_THREATS_START + Square::NUM;
const BLACK_THREATS_START: usize = WHITE_THREATS_END;
#[allow(dead_code)]
const BLACK_THREATS_END: usize = BLACK_THREATS_START + Square::NUM;
const SIDE_TO_MOVE_IDX: usize = NUM_FEATURES - 1;

// Bitset encoding
pub const BITS_PER_U64: usize = 64;

#[inline(always)]
pub fn encode_board(
    board: &Board,
    white_attacks: BitBoard,
    black_attacks: BitBoard,
    white_support: BitBoard,
    black_support: BitBoard,
    white_threats: BitBoard,
    black_threats: BitBoard,
) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    // Piece placements
    for sq in Square::ALL {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq as usize * 12 + piece_color_to_index(piece, color);
            features[offset] = 1.0;
        }
    }

    // White support
    for sq in white_support {
        features[WHITE_SUPPORT_START + sq as usize] = 1.0;
    }

    // Black support
    for sq in black_support {
        features[BLACK_SUPPORT_START + sq as usize] = 1.0;
    }

    // White space (controlled non-piece squares)
    let white_pieces = board.colors(Color::White);
    let white_space_bb = white_attacks & !white_pieces;
    for sq in white_space_bb {
        features[WHITE_SPACE_START + sq as usize] = 1.0;
    }

    // Black space
    let black_pieces = board.colors(Color::Black);
    let black_space_bb = black_attacks & !black_pieces;
    for sq in black_space_bb {
        features[BLACK_SPACE_START + sq as usize] = 1.0;
    }

    // White threats
    for sq in white_threats {
        features[WHITE_THREATS_START + sq as usize] = 1.0;
    }

    // Black threats
    for sq in black_threats {
        features[BLACK_THREATS_START + sq as usize] = 1.0;
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
    white_threats: BitBoard,
    black_threats: BitBoard,
) -> [u64; NUM_U64S] {
    let mut words = [0u64; NUM_U64S];

    // Piece placements
    for sq in Square::ALL {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq as usize * 12 + piece_color_to_index(piece, color);
            let word_idx = offset / 64;
            let bit_idx = offset % 64;
            words[word_idx] |= 1u64 << bit_idx;
        }
    }

    // White support
    for sq in white_support {
        let idx = WHITE_SUPPORT_START + sq as usize;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Black support
    for sq in black_support {
        let idx = BLACK_SUPPORT_START + sq as usize;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // White space (controlled non-piece squares)
    let white_pieces = board.colors(Color::White);
    let white_space_bb = white_attacks & !white_pieces;
    for sq in white_space_bb {
        let idx = WHITE_SPACE_START + sq as usize;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Black space
    let black_pieces = board.colors(Color::Black);
    let black_space_bb = black_attacks & !black_pieces;
    for sq in black_space_bb {
        let idx = BLACK_SPACE_START + sq as usize;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // White threats
    for sq in white_threats {
        let idx = WHITE_THREATS_START + sq as usize;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // Black threats
    for sq in black_threats {
        let idx = BLACK_THREATS_START + sq as usize;
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
