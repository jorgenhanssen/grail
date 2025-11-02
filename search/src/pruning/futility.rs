use evaluation::scores::MATE_VALUE;

pub const RAZOR_NEAR_MATE: i16 = MATE_VALUE - 200;

// Razor Pruning
// Aggressive forward pruning at very low depths
#[inline(always)]
pub fn razor_margin(depth: u8, base_margin: i16, depth_coefficient: i16) -> i16 {
    if depth == 0 {
        0
    } else {
        base_margin + depth_coefficient * (depth as i16 * depth as i16)
    }
}

#[inline(always)]
pub fn can_razor_prune(remaining_depth: u8, in_check: bool, max_depth: u8) -> bool {
    remaining_depth <= max_depth && remaining_depth > 0 && !in_check
}

// Forward Futility Pruning
// Prune moves when static eval + margin is below alpha (we're too far behind to catch up)
#[inline(always)]
pub fn futility_margin(depth: u8, base_margin: i16, depth_multiplier: i16) -> i16 {
    if depth == 0 {
        0
    } else {
        base_margin + (depth as i16 - 1) * depth_multiplier
    }
}

#[inline(always)]
pub fn can_futility_prune(remaining_depth: u8, in_check: bool, max_depth: u8) -> bool {
    remaining_depth <= max_depth && !in_check
}

// Reverse Futility Pruning (static beta pruning)
// Prune when static eval - margin exceeds beta (we're so far ahead opponent won't catch up)
#[inline(always)]
pub fn rfp_margin(
    depth: u8,
    base_margin: i16,
    depth_multiplier: i16,
    is_improving: bool,
    improving_bonus: i16,
) -> i16 {
    let margin = if depth == 0 {
        0
    } else {
        base_margin + (depth as i16 - 1) * depth_multiplier
    };

    if is_improving {
        margin - improving_bonus
    } else {
        margin
    }
}

#[inline(always)]
pub fn can_reverse_futility_prune(
    remaining_depth: u8,
    in_check: bool,
    is_pv_node: bool,
    max_depth: u8,
) -> bool {
    remaining_depth <= max_depth && remaining_depth > 0 && !in_check && !is_pv_node
}

// Delta Pruning (for quiescence search)
// Prune captures that can't possibly improve alpha even with the captured piece value
#[inline(always)]
pub fn can_delta_prune(in_check: bool, material_threshold: i16, total_material: i16) -> bool {
    !in_check && total_material >= material_threshold
}

