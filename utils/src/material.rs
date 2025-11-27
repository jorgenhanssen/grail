use cozy_chess::{BitBoard, Board, Color, Piece};

const LIGHT_SQUARES_MASK: u64 = 0x55AA55AA55AA55AA;

/// Get minor pieces (knights and bishops) for a color.
pub fn minors(board: &Board, color: Color) -> BitBoard {
    board.colored_pieces(color, Piece::Knight) | board.colored_pieces(color, Piece::Bishop)
}

/// Get major pieces (rooks and queens) for a color.
pub fn majors(board: &Board, color: Color) -> BitBoard {
    board.colored_pieces(color, Piece::Rook) | board.colored_pieces(color, Piece::Queen)
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

/// Check if the position has insufficient material for either side.
///
/// Returns true for dead drawn positions:
/// - K vs K
/// - K+N vs K (either side)
/// - K+B vs K (either side)
/// - K+B vs K+B with same-colored bishops
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

/// Compute game phase from piece counts.
///
/// Returns a value from 0.0 (endgame) to 1.0 (opening/middlegame).
/// Uses piece weights: N=1, B=1, R=2, Q=4, with max score of 24.
pub fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.len() + bishops.len() + 2 * rooks.len() + 4 * queens.len();

    (score.min(24) as f32) / 24.0
}

/// Check if position is prone to zugzwang (null-move pruning unsafe).
///
/// Returns true when:
/// - Side to move has only king and pawns
/// - Side to move has no pawns, no majors, and at most one minor piece
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = board.colors(board.side_to_move());
    let pawn_bits = board.pieces(Piece::Pawn) & side_bits;
    let king_bits = board.pieces(Piece::King) & side_bits;

    // Only king and pawns
    if side_bits == (pawn_bits | king_bits) {
        return true;
    }

    // No pawns, no majors, at most one minor
    let knight_bits = board.pieces(Piece::Knight) & side_bits;
    let bishop_bits = board.pieces(Piece::Bishop) & side_bits;
    let rook_bits = board.pieces(Piece::Rook) & side_bits;
    let queen_bits = board.pieces(Piece::Queen) & side_bits;

    let has_pawns = !pawn_bits.is_empty();
    let has_major = !(rook_bits | queen_bits).is_empty();
    let minor_count = (knight_bits | bishop_bits).len();

    !has_pawns && !has_major && minor_count <= 1
}
