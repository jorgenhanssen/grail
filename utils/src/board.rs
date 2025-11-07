use chess::{BitBoard, Board, ChessMove, Color, MoveGen, Piece, EMPTY};

const LIGHT_SQUARES_MASK: u64 = 0x55AA55AA55AA55AA;

#[inline(always)]
pub fn only_move(board: &Board) -> bool {
    let mut g = MoveGen::new_legal(board);
    matches!((g.next(), g.next()), (Some(_), None))
}

/// Checks if a specific color has insufficient material to force checkmate.
/// Returns true if the given color cannot possibly deliver checkmate.
/// This includes: K alone, K+N, K+B
#[inline(always)]
pub fn side_has_insufficient_material(board: &Board, color: Color) -> bool {
    let color_pieces = board.color_combined(color);

    let pawns = board.pieces(Piece::Pawn) & color_pieces;
    let rooks = board.pieces(Piece::Rook) & color_pieces;
    let queens = board.pieces(Piece::Queen) & color_pieces;

    // If this side has pawns, rooks, or queens, they can potentially win
    if (pawns | rooks | queens) != EMPTY {
        return false;
    }

    // Only king and minor pieces remain
    let knights = board.pieces(Piece::Knight) & color_pieces;
    let bishops = board.pieces(Piece::Bishop) & color_pieces;

    let minor_count = (knights | bishops).popcnt();

    // King alone, or K+N, or K+B cannot force mate
    minor_count <= 1
}

/// Checks if the position has insufficient material for either side to force checkmate.
/// Returns true for dead drawn positions like:
/// - K vs K
/// - K+N vs K (either side)
/// - K+B vs K (either side)
/// - K+B vs K+B with same-colored bishops
#[inline(always)]
pub fn has_insufficient_material(board: &Board) -> bool {
    let pawns = board.pieces(Piece::Pawn);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    // If there are any pawns, rooks, or queens, material is sufficient
    if (pawns | rooks | queens) != EMPTY {
        return false;
    }

    // Only kings and minor pieces remain
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);

    let white_knights = (white & knights).popcnt();
    let black_knights = (black & knights).popcnt();
    let white_bishops = (white & bishops).popcnt();
    let black_bishops = (black & bishops).popcnt();

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
        let white_on_light = (white & bishops & light_squares).popcnt() > 0;
        let black_on_light = (black & bishops & light_squares).popcnt() > 0;

        // Both on light or both on dark = insufficient material
        if white_on_light == black_on_light {
            return true;
        }
    }

    false
}

#[inline(always)]
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // Only king and pawns (common zugzwang scenario)
    if side_bits == (pawn_bits | king_bits) {
        return true;
    }

    // Positions with no pawns and no major pieces and at most
    // one minor piece are also prone to null-move failures.
    let knight_bits = *board.pieces(Piece::Knight) & side_bits;
    let bishop_bits = *board.pieces(Piece::Bishop) & side_bits;
    let rook_bits = *board.pieces(Piece::Rook) & side_bits;
    let queen_bits = *board.pieces(Piece::Queen) & side_bits;

    let has_pawns = pawn_bits.popcnt() > 0;
    let has_major = (rook_bits | queen_bits).popcnt() > 0;
    let minor_count = (knight_bits | bishop_bits).popcnt();

    !has_pawns && !has_major && minor_count <= 1
}

#[inline(always)]
pub fn game_phase(board: &Board) -> f32 {
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);

    let score = knights.popcnt() + bishops.popcnt() + 2 * rooks.popcnt() + 4 * queens.popcnt();

    (score.min(24) as f32) / 24.0
}

#[inline(always)]
pub fn gives_check(board: &Board, mv: ChessMove) -> bool {
    board.make_move_new(mv).checkers().popcnt() > 0
}
