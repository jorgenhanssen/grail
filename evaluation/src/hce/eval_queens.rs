use super::HCEConfig;
use crate::hce::context::EvalContext;
use chess::Color;

#[inline(always)]
pub(super) fn evaluate(_ctx: &EvalContext, _color: Color, _config: &HCEConfig) -> i16 {
    0
}
