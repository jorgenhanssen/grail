use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::{Color, Piece};

/// Evaluate space advantage based on space controlled
/// Uses cached attack map from Position (shared with threat detection)
#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let space = ctx.position.space_for(color);
    config.space_multiplier * space
}

/// Evaluate piece coordination - bonuses for defended pieces
/// Defended pieces are more stable and can be more aggressive
#[inline(always)]
pub(super) fn evaluate_support(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let support = ctx.position.support_for(color);

    let my_pieces = board.color_combined(color);
    let knights = board.pieces(Piece::Knight) & my_pieces;
    let bishops = board.pieces(Piece::Bishop) & my_pieces;
    let rooks = board.pieces(Piece::Rook) & my_pieces;
    let queens = board.pieces(Piece::Queen) & my_pieces;

    let minors = knights | bishops;
    let majors = rooks | queens;

    let supported_minors = (support & minors).popcnt() as i16;
    let supported_majors = (support & majors).popcnt() as i16;

    config.supported_minor_bonus * supported_minors
        + config.supported_major_bonus * supported_majors
}
