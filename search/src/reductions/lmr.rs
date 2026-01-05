/// Late Move Reductions: reduce depth for late quiet moves.
/// Reduction based on ln(depth) * ln(move_index).
///
/// <https://www.chessprogramming.org/Late_Move_Reductions>
#[allow(clippy::too_many_arguments)]
pub fn lmr(remaining_depth: u8, move_index: i32, divisor: f32) -> u8 {
    if remaining_depth == 0 || move_index == 0 {
        return 0;
    }

    let depth_factor = (remaining_depth as f32).ln();
    let move_factor = (move_index as f32 / divisor).ln();

    // 0.5 for rounding
    (0.5 + (depth_factor * move_factor)) as u8
}
