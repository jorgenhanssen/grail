use crate::scores::MATE_VALUE;
use cozy_chess::{Move, Piece};
use utils::{Node, captured_piece, piece_value};

use crate::utils::{Bounds, see::see};

use super::Searcher;

const NEAR_MATE_VALUE: i16 = MATE_VALUE - 200;

/// Mate distance pruning: adjusts alpha-beta bounds based on the maximum
/// possible mate score at current ply. Returns true if the search can be pruned.
///
/// Example: A mate found at ply P is at least P plies from root, so:
/// - Best possible score: MATE_VALUE - ply (mate-in-P)
/// - Worst possible score: -(MATE_VALUE - ply) (mated-in-P)
///
/// <https://www.chessprogramming.org/Mate_Distance_Pruning>
pub(super) fn mate_distance_prune(bounds: &mut Bounds, ply: u8) -> bool {
    let mate_in_ply = MATE_VALUE - ply as i16;
    let mated_in_ply = -(MATE_VALUE - ply as i16);

    bounds.alpha = bounds.alpha.max(mated_in_ply);
    bounds.beta = bounds.beta.min(mate_in_ply);

    bounds.alpha >= bounds.beta
}

impl Searcher {
    /// Futility pruning: skip moves unlikely to raise alpha based on static eval + margin.
    ///
    /// <https://www.chessprogramming.org/Futility_Pruning>
    pub(super) fn try_futility_prune(
        &self,
        depth: u8,
        in_check: bool,
        is_tactical: bool,
        alpha: i16,
        static_eval: i16,
    ) -> bool {
        if depth > self.config.futility_max_depth.value || in_check {
            return false;
        }
        let margin = self.config.futility_base_margin.value
            + depth.saturating_sub(1) as i16 * self.config.futility_depth_multiplier.value;
        !is_tactical && static_eval + margin <= alpha
    }

    /// Razoring: if eval is far below alpha, drop into qsearch to verify and return early.
    ///
    /// <https://www.chessprogramming.org/Razoring>
    pub(super) fn try_razor_prune(
        &mut self,
        node: &Node,
        depth: u8,
        alpha: i16,
        ply: u8,
        in_check: bool,
        static_eval: i16,
    ) -> Option<i16> {
        if depth == 0 || depth > self.config.razor_max_depth.value || in_check {
            return None;
        }
        let margin = self.config.razor_base_margin.value
            + self.config.razor_depth_coefficient.value * (depth as i16 * depth as i16);
        if static_eval >= alpha - margin {
            return None;
        }
        let value = self.quiescence_search(node, Bounds::null(alpha - 1), ply);
        if value < alpha && value.abs() < NEAR_MATE_VALUE {
            Some(value)
        } else {
            None
        }
    }

    /// SEE pruning: skip moves where the resulting exchange sequence loses material.
    /// Applies near the leaves where there's not enough depth to discover this naturally.
    ///
    /// <https://www.chessprogramming.org/Static_Exchange_Evaluation>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_see_prune(
        &self,
        node: &Node,
        m: Move,
        moved_piece: Piece,
        is_capture: bool,
        depth: u8,
        in_check: bool,
        is_pv_move: bool,
        alpha: i16,
        static_eval: i16,
    ) -> bool {
        if is_pv_move || in_check || node.is_pv() || m.promotion.is_some() {
            return false;
        }

        let board = node.board();

        if is_capture {
            if depth > self.config.see_capture_max_depth.value {
                return false;
            }

            let captured = captured_piece(board, m).unwrap();
            let captured_value = piece_value(captured);
            let attacker_value = piece_value(moved_piece);

            // Only run it on questionable captures (SEE is expensive):
            // Skip if victim >= attacker (looks good) or attacker is trivial/invaluable
            if captured_value >= attacker_value
                || attacker_value < self.config.see_capture_min_attacker_value.value
            {
                return false;
            }

            // When we're behind on eval we need captures to actually win material,
            // but tolerate more at higher depths since there's room to recover.
            let eval_gap = alpha - static_eval;
            let depth_margin = self.config.see_capture_depth_margin.value * (depth as i16);
            let threshold = -(eval_gap.max(0) + depth_margin);

            !see(board, m, threshold)
        } else {
            if depth > self.config.see_quiet_max_depth.value {
                return false;
            }

            // Catch quiet moves that walk into losing exchanges (hanging, etc)
            // Tolerate more at higher depths since the search can correct mistakes.
            let threshold = -(self.config.see_quiet_depth_multiplier.value * depth as i16);

            !see(board, m, threshold)
        }
    }

    /// Null move pruning: give opponent a free move; if we still beat beta, prune the subtree.
    /// Includes verification search at high depths to avoid zugzwang.
    ///
    /// <https://www.chessprogramming.org/Null_Move_Pruning>
    pub(super) fn try_null_move_prune(
        &mut self,
        node: &Node,
        depth: u8,
        ply: u8,
        bounds: Bounds,
        in_check: bool,
        try_null_move: bool,
        static_eval: Option<i16>,
    ) -> Option<i16> {
        if !try_null_move
            || in_check
            || !node.is_cut()
            || depth < self.config.nmp_min_depth.value
            || node.is_zugzwang()
        {
            return None;
        }

        let nm_child = node.create_null_move_child()?;

        // Deeper positions get more reduction
        let base_r = self.config.nmp_base_reduction.value;
        let mut r = base_r + (depth / self.config.nmp_depth_divisor.value);

        if let Some(se) = static_eval {
            let margin = self.config.nmp_eval_margin.value;
            if se >= bounds.beta + margin {
                // Strong positions get extra reduction
                r = r.saturating_add(1);
            } else if se <= bounds.beta - margin {
                // Weak positions get less reduction
                r = r.saturating_sub(1).max(base_r);
            }
        }

        if r >= depth {
            r = depth.saturating_sub(1).max(base_r);
        }

        // Null window around beta for the null move search
        let null_bounds = Bounds::null(-bounds.beta);
        let reduced_depth = depth.saturating_sub(r + 1);

        // Do a reduced depth null search to check if our position is still good enough
        self.search_stack.push_node(&nm_child);
        let null_value = -self.search_node(&nm_child, reduced_depth, ply + 1, null_bounds, false);
        self.search_stack.pop();

        if null_value < bounds.beta || null_value.abs() >= NEAR_MATE_VALUE {
            return None;
        }

        // At high depths, verify the null-move result with NMP disabled.
        if depth >= self.config.nmp_verify_depth.value {
            let v = self.search_node(
                node,
                reduced_depth,
                ply,
                Bounds::null(bounds.beta - 1),
                false,
            );
            if v < bounds.beta {
                return None;
            }
        }

        self.shared.tt().store(
            node.hash(),
            ply,
            depth.saturating_sub(r),
            bounds.beta,
            None,
            bounds.alpha,
            bounds.beta,
            None,
        );
        Some(bounds.beta)
    }

    /// Reverse futility pruning: if static eval - margin >= beta, the position is too good to search.
    ///
    /// <https://www.chessprogramming.org/Reverse_Futility_Pruning>
    pub(super) fn try_reverse_futility_prune(
        &mut self,
        node: &Node,
        depth: u8,
        in_check: bool,
        static_eval: i16,
        bounds: Bounds,
        ply: u8,
        is_improving: bool,
    ) -> Option<i16> {
        if depth == 0 || depth > self.config.rfp_max_depth.value || in_check || node.is_pv() {
            return None;
        }

        let mut margin = self.config.rfp_base_margin.value
            + (depth as i16 - 1) * self.config.rfp_depth_multiplier.value;
        if is_improving {
            margin -= self.config.rfp_improving_bonus.value;
        }

        if static_eval - margin >= bounds.beta && static_eval.abs() < NEAR_MATE_VALUE {
            self.shared.tt().store(
                node.hash(),
                ply,
                0,
                bounds.beta,
                Some(static_eval),
                bounds.alpha,
                bounds.beta,
                None,
            );
            return Some(bounds.beta);
        }
        None
    }

    /// Late move pruning: near the horizon, skip quiet moves beyond a count threshold.
    /// As iterative deepening extends the horizon, nodes that were at the frontier open up
    /// to search more moves. This forms a right-triangle search shape, narrow tip at the
    /// current horizon, widening toward the root.
    ///
    /// <https://www.chessprogramming.org/Futility_Pruning#MoveCountBasedPruning>
    pub(super) fn should_lmp_prune(
        &self,
        node: &Node,
        mv: Move,
        in_check: bool,
        depth: u8,
        move_index: i32,
        is_improving: bool,
    ) -> bool {
        let is_cap = node.is_capture(mv);
        let is_promotion = mv.promotion == Some(Piece::Queen);

        if in_check
            || node.is_pv()
            || is_cap
            || is_promotion
            || depth > self.config.lmp_max_depth.value
        {
            return false;
        }

        let base = self.config.lmp_base_moves.value;
        let mult = self.config.lmp_depth_multiplier.value;
        let mut limit = base + (depth as i32 * (depth as i32 + mult)) / 2;

        // Be more aggressive (prune earlier) when position isn't improving
        if !is_improving {
            limit = (limit * self.config.lmp_improving_reduction.value) / 100;
        }

        move_index > limit
    }
}
