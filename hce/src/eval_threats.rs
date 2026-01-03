use super::HCEConfig;
use crate::context::EvalContext;
use cozy_chess::Color;

/// Evaluate threats based on attacks on opponent pieces
/// Uses cached attack map from Node (shared with space evaluation)
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    // Count number of threats against opponent pieces
    let num_threats = ctx.node.threats_for(!color);
    config.threats_multiplier * num_threats.len() as i16
}
