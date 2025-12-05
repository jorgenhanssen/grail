use super::HCEConfig;
use crate::context::EvalContext;
use cozy_chess::Color;
use utils::{majors, minors};

/// Evaluate space advantage based on space controlled
/// Uses cached attack map from Position (shared with threat detection)
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let space = ctx.position.space_for(color);
    config.space_multiplier * space
}

/// Evaluate piece coordination - bonuses for defended pieces
/// Defended pieces are more stable and can be more aggressive
/// TODO: Consider moving support to own file
pub(super) fn evaluate_support(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let support = ctx.position.support_for(color);

    let supported_minors = (support & minors(board, color)).len() as i16;
    let supported_majors = (support & majors(board, color)).len() as i16;

    config.supported_minor_bonus * supported_minors
        + config.supported_major_bonus * supported_majors
}
