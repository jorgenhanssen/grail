use chess::{Board, Color, Piece, ALL_SQUARES};

pub const NUM_FEATURES: usize = 773;
pub const NUM_U64S: usize = (NUM_FEATURES + 63) / 64;

const SIDE_TO_MOVE_IDX: usize = 768;
const CASTLE_BASE_IDX: usize = 769;

/// Returns a one-hot encoding of the board without en-passant or promotion availability.
///
/// Layout (all entries are 0.0 or 1.0):
///
///  1) Piece placements [0..768):
///       For each of the 64 squares, 12 piece channels:
///         0 = White Pawn
///         1 = White Knight
///         2 = White Bishop
///         3 = White Rook
///         4 = White Queen
///         5 = White King
///         6 = Black Pawn
///         7 = Black Knight
///         8 = Black Bishop
///         9 = Black Rook
///         10 = Black Queen
///         11 = Black King
///       Index = square_index * 12 + piece_channel.
///       If `square` has `piece`, features[index] = 1.0.
///
///  2) Side to move [768]:
///       1.0 if White to move, else 0.0
///
///  3) Castling rights [769..773]:
///       - 769 = White can castle kingside
///       - 770 = White can castle queenside
///       - 771 = Black can castle kingside
///       - 772 = Black can castle queenside
///
///  Total features = 773.  (No en-passant, no promotion bits.)

#[inline(always)]
pub fn encode_board(board: &Board) -> [f32; NUM_FEATURES] {
    // 64 squares * 12 piece-types = 768
    // + 1 (side to move)
    // + 4 (castling rights)
    // = 773 total
    let mut features = [0f32; NUM_FEATURES];

    // 1) Piece placements [0..768)
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq.to_index() * 12 + piece_color_to_index(piece, color);
            features[offset] = 1.0;
        }
    }

    if board.side_to_move() == Color::White {
        features[SIDE_TO_MOVE_IDX] = 1.0;
    }

    // 3) Castling rights [769..772]
    let wcr = board.castle_rights(Color::White);
    let bcr = board.castle_rights(Color::Black);

    if wcr.has_kingside() {
        features[CASTLE_BASE_IDX] = 1.0;
    }
    if wcr.has_queenside() {
        features[CASTLE_BASE_IDX + 1] = 1.0;
    }
    if bcr.has_kingside() {
        features[CASTLE_BASE_IDX + 2] = 1.0;
    }
    if bcr.has_queenside() {
        features[CASTLE_BASE_IDX + 3] = 1.0;
    }

    features
}

#[inline(always)]
pub fn encode_board_bitset(board: &Board) -> [u64; NUM_U64S] {
    let mut words = [0u64; NUM_U64S];

    // 1) Piece placements [0..768)
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq.to_index() * 12 + piece_color_to_index(piece, color);
            let word_idx = offset / 64;
            let bit_idx = offset % 64;
            words[word_idx] |= 1u64 << bit_idx;
        }
    }

    // 2) Side to move [768]
    if board.side_to_move() == Color::White {
        let idx = SIDE_TO_MOVE_IDX;
        words[idx / 64] |= 1u64 << (idx % 64);
    }

    // 3) Castling rights [769..772]
    let wcr = board.castle_rights(Color::White);
    let bcr = board.castle_rights(Color::Black);

    if wcr.has_kingside() {
        let idx = CASTLE_BASE_IDX;
        words[idx / 64] |= 1u64 << (idx % 64);
    }
    if wcr.has_queenside() {
        let idx = CASTLE_BASE_IDX + 1;
        words[idx / 64] |= 1u64 << (idx % 64);
    }
    if bcr.has_kingside() {
        let idx = CASTLE_BASE_IDX + 2;
        words[idx / 64] |= 1u64 << (idx % 64);
    }
    if bcr.has_queenside() {
        let idx = CASTLE_BASE_IDX + 3;
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
