use chess::{Board, Color, Piece};

/// Applies scaling factors for drawish endgames
#[inline(always)]
pub(super) fn apply(board: &Board, cp: i16, phase: f32) -> i16 {
    // Gate to endgame; avoid distorting middlegames
    if phase >= 0.25 {
        return cp;
    }
    let white = Color::White;
    let black = Color::Black;
    let w_pawns_i16 = (board.pieces(Piece::Pawn) & board.color_combined(white)).popcnt() as i16;
    let b_pawns_i16 = (board.pieces(Piece::Pawn) & board.color_combined(black)).popcnt() as i16;
    let w_pawns = (board.pieces(Piece::Pawn) & board.color_combined(white)).popcnt();
    let b_pawns = (board.pieces(Piece::Pawn) & board.color_combined(black)).popcnt();
    let w_knights = (board.pieces(Piece::Knight) & board.color_combined(white)).popcnt();
    let b_knights = (board.pieces(Piece::Knight) & board.color_combined(black)).popcnt();
    let w_bishops = (board.pieces(Piece::Bishop) & board.color_combined(white)).popcnt();
    let b_bishops = (board.pieces(Piece::Bishop) & board.color_combined(black)).popcnt();
    let w_rooks = (board.pieces(Piece::Rook) & board.color_combined(white)).popcnt();
    let b_rooks = (board.pieces(Piece::Rook) & board.color_combined(black)).popcnt();
    let w_queens = (board.pieces(Piece::Queen) & board.color_combined(white)).popcnt();
    let b_queens = (board.pieces(Piece::Queen) & board.color_combined(black)).popcnt();

    let mut scale = 1.0f32;

    // Opposite-colored bishops endgame
    if w_bishops == 1
        && b_bishops == 1
        && w_knights == 0
        && b_knights == 0
        && w_rooks == 0
        && b_rooks == 0
        && w_queens == 0
        && b_queens == 0
    {
        let w_bsq = (board.pieces(Piece::Bishop) & board.color_combined(white))
            .into_iter()
            .next()
            .unwrap()
            .to_index() as u8;
        let b_bsq = (board.pieces(Piece::Bishop) & board.color_combined(black))
            .into_iter()
            .next()
            .unwrap()
            .to_index() as u8;
        if is_light_square(w_bsq) != is_light_square(b_bsq) {
            let pawn_diff = (w_pawns_i16 - b_pawns_i16).abs() as f32;
            let ocb_scale = (0.55 + 0.05 * pawn_diff).min(0.9);
            scale = scale.min(ocb_scale);
        }
    }

    // Two knights vs bare king (insufficient material)
    if (w_knights == 2
        && (w_bishops + w_rooks + w_queens + w_pawns) == 0
        && (b_knights + b_bishops + b_rooks + b_queens + b_pawns) == 0)
        || (b_knights == 2
            && (b_bishops + b_rooks + b_queens + b_pawns) == 0
            && (w_knights + w_bishops + w_rooks + w_queens + w_pawns) == 0)
    {
        scale = scale.min(0.1);
    }

    // Single minor vs bare king
    if (w_pawns == 0 && b_pawns == 0 && (w_rooks + w_queens) == 0 && (b_rooks + b_queens) == 0)
        && (((w_knights + w_bishops) == 1 && (b_knights + b_bishops) == 0)
            || ((b_knights + b_bishops) == 1 && (w_knights + w_bishops) == 0))
    {
        scale = scale.min(0.2);
    }

    // Rook vs minor
    if (w_queens + b_queens) == 0
        && (w_rooks + b_rooks) == 1
        && (w_knights + w_bishops + b_knights + b_bishops) == 1
        && (w_pawns + b_pawns) == 0
    {
        scale = scale.min(0.75);
    }

    // Rook + minor vs rook
    if (w_queens + b_queens) == 0
        && ((w_rooks == 1
            && b_rooks == 1
            && (w_knights + w_bishops) == 1
            && (b_knights + b_bishops) == 0)
            || (b_rooks == 1
                && w_rooks == 1
                && (b_knights + b_bishops) == 1
                && (w_knights + w_bishops) == 0))
        && (w_pawns + b_pawns) == 0
    {
        scale = scale.min(0.8);
    }

    ((cp as f32) * scale).round() as i16
}

#[inline(always)]
fn is_light_square(idx: u8) -> bool {
    let file = (idx % 8) as i32;
    let rank = (idx / 8) as i32;
    ((file + rank) & 1) == 0
}
