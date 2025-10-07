use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::Color;

/// Evaluate space advantage based on space controlled
/// Uses cached attack map from Position (shared with threat detection)
#[inline(always)]
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let attack_map = ctx.position.attack_map();
    let space = attack_map.space_for(color);
    config.space_multiplier * space
}
