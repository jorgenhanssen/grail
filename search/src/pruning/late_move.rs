use chess::{Board, ChessMove, Piece};

// Late Move Pruning (LMP)
// Conservative triangular limits per remaining depth for how many quiet moves
// are searched before pruning subsequent quiets in non-PV, non-check nodes.
#[inline(always)]
pub fn lmp_move_limit(depth: u8, base_moves: i32, depth_multiplier: i32) -> i32 {
    // Triangular number pattern: base + depth * (depth + multiplier) / 2
    base_moves + (depth as i32 * (depth as i32 + depth_multiplier)) / 2
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn should_lmp_prune(
    board: &Board,
    mv: ChessMove,
    in_check: bool,
    is_pv_node: bool,
    remaining_depth: u8,
    move_index: i32,
    is_improving: bool,
    max_depth: u8,
    base_moves: i32,
    depth_multiplier: i32,
    improving_reduction: i32,
) -> bool {
    let is_capture = board.piece_on(mv.get_dest()).is_some();
    let is_promotion = mv.get_promotion() == Some(Piece::Queen);

    if in_check || is_pv_node || is_capture || is_promotion || remaining_depth > max_depth {
        return false;
    }

    let mut limit = lmp_move_limit(remaining_depth, base_moves, depth_multiplier);

    // Be more aggressive (prune earlier) when position isn't improving
    if !is_improving {
        limit = (limit * improving_reduction) / 100;
    }

    move_index > limit
}
