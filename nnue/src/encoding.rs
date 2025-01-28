use chess::{Board, Color, Piece, ALL_SQUARES};

pub const NUM_FEATURES: usize = 773;

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
pub fn encode_board(board: &Board) -> [i8; NUM_FEATURES] {
    // 64 squares * 12 piece-types = 768
    // + 1 (side to move)
    // + 4 (castling rights)
    // = 773 total
    let mut features = [0i8; NUM_FEATURES];

    // 1) Piece placements [0..768)
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let piece_channel = piece_color_to_index(piece, color);
            let sq_index = sq.to_index(); // 0..63
            let offset = sq_index * 12 + piece_channel;
            features[offset] = 1;
        }
    }

    // 2) Side to move [768]
    let side_to_move_idx = 768;
    if board.side_to_move() == Color::White {
        features[side_to_move_idx] = 1;
    }

    // 3) Castling rights [769..772]
    let castle_base = side_to_move_idx + 1;
    features[castle_base + 0] = if board.castle_rights(Color::White).has_kingside() {
        1
    } else {
        0
    };
    features[castle_base + 1] = if board.castle_rights(Color::White).has_queenside() {
        1
    } else {
        0
    };
    features[castle_base + 2] = if board.castle_rights(Color::Black).has_kingside() {
        1
    } else {
        0
    };
    features[castle_base + 3] = if board.castle_rights(Color::Black).has_queenside() {
        1
    } else {
        0
    };

    features
}

/// Maps (Piece, Color) to a channel index in [0..11].
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
