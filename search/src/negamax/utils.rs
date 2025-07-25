use chess::{Board, ChessMove, Piece};
use evaluation::{piece_value, scores::MATE_VALUE, total_material};
use uci::commands::Score;

pub const RAZOR_MAX_DEPTH: u8 = 3;
pub const RAZOR_NEAR_MATE: i16 = MATE_VALUE - 200;

// Margins from Stockfish: https://www.chessprogramming.org/Razoring#Stockfish
pub const RAZOR_MARGINS: [i16; RAZOR_MAX_DEPTH as usize + 1] = {
    let mut margins = [0i16; RAZOR_MAX_DEPTH as usize + 1];
    let mut depth = 1;
    while depth <= RAZOR_MAX_DEPTH as i16 {
        margins[depth as usize] = 512 + 293 * (depth * depth);
        depth += 1;
    }
    margins
};

#[inline(always)]
pub fn see_naive(board: &Board, capture_move: ChessMove) -> i16 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        piece_value(captured_piece) - piece_value(capturing_piece)
    } else {
        0
    }
}

#[inline(always)]
pub fn lmr(remaining_depth: u8, tactical: bool, move_index: i32) -> u8 {
    if tactical || remaining_depth < 3 || move_index < 3 {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32).ln();

    let reduction = (depth_factor * move_factor / 2.5).round() as u8;

    // Clamp between 0 and half the remaining depth
    let half_depth = (remaining_depth / 2).max(1);
    reduction.min(half_depth).max(0)
}

#[inline(always)]
pub fn convert_mate_score(score: i16, pv: &Vec<ChessMove>) -> Score {
    let mate_in = (pv.len() as i16 + 1) / 2;

    if score > 0 {
        Score::Mate(mate_in)
    } else {
        Score::Mate(-mate_in)
    }
}

#[inline(always)]
pub fn convert_centipawn_score(score: i16) -> Score {
    Score::Centipawns(score)
}

pub fn can_delta_prune(board: &Board, in_check: bool) -> bool {
    !in_check && total_material(board) >= 1500
}

#[inline(always)]
pub fn can_null_move_prune(board: &Board, remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth >= 3 && !in_check && !is_zugzwang(board)
}

#[inline(always)]
pub fn can_razor_prune(remaining_depth: u8, in_check: bool) -> bool {
    remaining_depth <= RAZOR_MAX_DEPTH && remaining_depth > 0 && !in_check
}

#[inline(always)]
pub fn is_zugzwang(board: &Board) -> bool {
    let side_bits = *board.color_combined(board.side_to_move());
    let pawn_bits = *board.pieces(Piece::Pawn) & side_bits;
    let king_bits = *board.pieces(Piece::King) & side_bits;

    // King + pawn endgame is typical zugzwang
    (side_bits ^ pawn_bits ^ king_bits).popcnt() == 0
}
