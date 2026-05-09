use cozy_chess::{BitBoard, Board, Color, Piece};

const LIGHT_SQUARES_MASK: u64 = 0x55AA55AA55AA55AA;

// Standard piece values (centipawns), indexed by Piece enum order
const PIECE_VALUES: [i16; Piece::NUM] = [
    100, // Pawn
    320, // Knight
    330, // Bishop
    500, // Rook
    900, // Queen
    0,   // King
];

pub const PAWN_VALUE: i16 = PIECE_VALUES[Piece::Pawn as usize];
pub const KNIGHT_VALUE: i16 = PIECE_VALUES[Piece::Knight as usize];
pub const BISHOP_VALUE: i16 = PIECE_VALUES[Piece::Bishop as usize];
pub const ROOK_VALUE: i16 = PIECE_VALUES[Piece::Rook as usize];
pub const QUEEN_VALUE: i16 = PIECE_VALUES[Piece::Queen as usize];

// Piece groups
pub const PAWN_PIECES: [Piece; 1] = [Piece::Pawn];
pub const MINOR_PIECES: [Piece; 2] = [Piece::Knight, Piece::Bishop];
pub const MAJOR_PIECES: [Piece; 2] = [Piece::Rook, Piece::Queen];
pub const NON_PAWN_PIECES: [Piece; 5] = [
    Piece::King,
    Piece::Knight,
    Piece::Bishop,
    Piece::Rook,
    Piece::Queen,
];

/// Get the value of a piece in centipawns.
pub fn piece_value(piece: Piece) -> i16 {
    PIECE_VALUES[piece as usize]
}

/// Sum the value of all pieces on the board.
pub fn total_material(board: &Board) -> i16 {
    let mut material = 0;
    for piece in Piece::ALL {
        material += piece_value(piece) * (board.pieces(piece).len() as i16);
    }
    material
}

/// Get minor pieces (knights and bishops) for a color.
pub fn minors(board: &Board, color: Color) -> BitBoard {
    board.colored_pieces(color, Piece::Knight) | board.colored_pieces(color, Piece::Bishop)
}

/// Get major pieces (rooks and queens) for a color.
pub fn majors(board: &Board, color: Color) -> BitBoard {
    board.colored_pieces(color, Piece::Rook) | board.colored_pieces(color, Piece::Queen)
}

/// Cap evaluation based on insufficient material.
/// If a side cannot possibly win, cap eval so they can't be "winning".
pub fn cap_eval_by_material(board: &Board, score: i16) -> i16 {
    let mut capped = score;
    if side_has_insufficient_material(board, Color::White) {
        capped = capped.min(0);
    }
    if side_has_insufficient_material(board, Color::Black) {
        capped = capped.max(0);
    }
    capped
}

/// Check if a specific color has insufficient material to force checkmate.
pub fn side_has_insufficient_material(board: &Board, color: Color) -> bool {
    let color_pieces = board.colors(color);

    let pawns = board.pieces(Piece::Pawn) & color_pieces;
    let rooks = board.pieces(Piece::Rook) & color_pieces;
    let queens = board.pieces(Piece::Queen) & color_pieces;

    if !(pawns | rooks | queens).is_empty() {
        return false;
    }

    let knights = board.pieces(Piece::Knight) & color_pieces;
    let bishops = board.pieces(Piece::Bishop) & color_pieces;
    let minor_count = (knights | bishops).len();

    minor_count <= 1
}

/// Check if the position is a draw by material.
pub fn has_insufficient_material(board: &Board) -> bool {
    let pawns = board.pieces(Piece::Pawn);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    if !(pawns | rooks | queens).is_empty() {
        return false;
    }

    let white = board.colors(Color::White);
    let black = board.colors(Color::Black);
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);

    let white_knights = (white & knights).len();
    let black_knights = (black & knights).len();
    let white_bishops = (white & bishops).len();
    let black_bishops = (black & bishops).len();

    let white_minors = white_knights + white_bishops;
    let black_minors = black_knights + black_bishops;

    // K vs K
    if white_minors == 0 && black_minors == 0 {
        return true;
    }

    // K+N vs K or K vs K+N
    if white_minors == 1 && white_knights == 1 && black_minors == 0 {
        return true;
    }
    if black_minors == 1 && black_knights == 1 && white_minors == 0 {
        return true;
    }

    // K+B vs K or K vs K+B
    if white_minors == 1 && white_bishops == 1 && black_minors == 0 {
        return true;
    }
    if black_minors == 1 && black_bishops == 1 && white_minors == 0 {
        return true;
    }

    // K+B vs K+B with bishops on same color squares
    if white_bishops == 1 && black_bishops == 1 && white_minors == 1 && black_minors == 1 {
        let light_squares = BitBoard(LIGHT_SQUARES_MASK);
        let white_on_light = !(white & bishops & light_squares).is_empty();
        let black_on_light = !(black & bishops & light_squares).is_empty();

        if white_on_light == black_on_light {
            return true;
        }
    }

    false
}

/// Check if position is prone to zugzwang (based on Stockfish).
///
/// Returns true when side to move has no pieces (only king and pawns).
pub fn is_zugzwang(board: &Board) -> bool {
    let stm = board.side_to_move();
    (minors(board, stm) | majors(board, stm)).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insufficient_material() {
        let cases: &[(&str, bool)] = &[
            ("k7/8/8/8/8/8/8/K7 w - - 0 1", true),
            ("k7/8/8/8/8/8/8/KN6 w - - 0 1", true),
            ("kn6/8/8/8/8/8/8/K7 w - - 0 1", true),
            ("k7/8/8/8/8/8/8/KB6 w - - 0 1", true),
            ("k7/8/8/8/8/8/8/KNN5 w - - 0 1", false),
            ("k7/p7/8/8/8/8/8/K7 w - - 0 1", false),
            ("k7/8/8/8/8/8/8/KR6 w - - 0 1", false),
        ];
        for (fen, expected) in cases {
            let board: Board = fen.parse().unwrap();
            assert_eq!(has_insufficient_material(&board), *expected, "{fen}");
        }
    }

    #[test]
    fn zugzwang_when_no_pieces() {
        let cases: &[(&str, bool)] = &[
            ("k7/8/8/8/8/8/8/K7 w - - 0 1", true),
            ("k7/8/8/8/8/8/P7/K7 w - - 0 1", true),
            ("k7/8/8/8/8/8/8/KR6 w - - 0 1", false),
            ("k7/8/8/8/8/8/8/KQ6 w - - 0 1", false),
        ];
        for (fen, expected) in cases {
            let board: Board = fen.parse().unwrap();
            assert_eq!(is_zugzwang(&board), *expected, "{fen}");
        }
    }
}
