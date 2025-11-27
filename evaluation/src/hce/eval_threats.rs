use super::HCEConfig;
use crate::hce::context::EvalContext;
use cozy_chess::Color;

/// Evaluate space advantage based on space controlled
/// Uses cached attack map from Position (shared with threat detection)
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    // Count number of threats to opponent pieces
    let num_threats = ctx.position.threats_for(!color);
    config.threats_multiplier * num_threats.len() as i16
}
