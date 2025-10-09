use chess::{Board, Color, Piece, ALL_SQUARES, NUM_COLORS, NUM_PIECES, NUM_SQUARES};

// One-hot board encoding:
// - 768 piece placement features (64 squares × 6 pieces × 2 colors)
// - 1 side-to-move bit
pub const NUM_FEATURES: usize = NUM_SQUARES * NUM_PIECES * NUM_COLORS + 1;

pub const NUM_U64S: usize = NUM_FEATURES.div_ceil(64);

const SIDE_TO_MOVE_IDX: usize = NUM_FEATURES - 1;

/// Encodes a board position as a one-hot feature vector.
#[inline(always)]
pub fn encode_board(board: &Board) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    // Piece placements
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq.to_index() * 12 + piece_color_to_index(piece, color);
            features[offset] = 1.0;
        }
    }

    // Side to move
    if board.side_to_move() == Color::White {
        features[SIDE_TO_MOVE_IDX] = 1.0;
    }

    features
}

/// Encodes a board position as a bitset (for incremental NNUE updates).
#[inline(always)]
pub fn encode_board_bitset(board: &Board) -> [u64; NUM_U64S] {
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
