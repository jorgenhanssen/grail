// Late Move Reduction (LMR)
// Reduces search depth for moves that are likely to be bad (searched late in move ordering)
#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn lmr(
    remaining_depth: u8,
    tactical: bool,
    move_index: i32,
    is_pv_move: bool,
    is_improving: bool,
    min_depth: u8,
    divisor: f32,
    max_reduction_ratio: f32,
) -> u8 {
    if tactical || remaining_depth < min_depth || is_pv_move {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32).ln();

    let mut reduction = (depth_factor * move_factor / divisor).round() as u8;

    if !is_improving {
        reduction = reduction.saturating_add(1);
    }

    let max_reduction = (remaining_depth as f32 * max_reduction_ratio) as u8;
    reduction.min(max_reduction)
}
