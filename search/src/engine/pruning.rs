use cozy_chess::{Move, Piece};
use utils::{Node, captured_piece, piece_value};

use crate::{
    pruning::{
        RAZOR_NEAR_MATE, can_futility_prune, can_null_move_prune, can_razor_prune,
        can_reverse_futility_prune, futility_margin, null_move_reduction, razor_margin, rfp_margin,
    },
    utils::{Bounds, see::see},
};

use super::Engine;

impl Engine {
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
        if !can_futility_prune(depth, in_check, self.config.futility_max_depth.value) {
            return false;
        }
        let margin = futility_margin(
            depth,
            self.config.futility_base_margin.value,
            self.config.futility_depth_multiplier.value,
        );
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
        if !can_razor_prune(depth, in_check, self.config.razor_max_depth.value) {
            return None;
        }
        // If static eval already near/above alpha threshold, do not razor
        let margin = razor_margin(
            depth,
            self.config.razor_base_margin.value,
            self.config.razor_depth_coefficient.value,
        );
        if static_eval >= alpha - margin {
            return None;
        }
        // Q search with null window
        let (value, _) = self.quiescence_search(node, Bounds::null(alpha - 1), ply);
        if value < alpha && value.abs() < RAZOR_NEAR_MATE {
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
    /// Includes verification search at low depths to avoid zugzwang.
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
        if !(try_null_move
            && can_null_move_prune(node, depth, in_check, self.config.nmp_min_depth.value))
        {
            return None;
        }

        let nm_child = node.create_null_move_child()?;

        // Calculate reduction based on remaining depth and static eval
        let reduction: u8 = null_move_reduction(
            depth,
            static_eval,
            bounds.beta,
            self.config.nmp_base_reduction.value,
            self.config.nmp_depth_divisor.value,
            self.config.nmp_eval_margin.value,
        );

        // Null window around beta for the null move search
        let null_bounds = Bounds::null(-bounds.beta);

        // Do a reduced depth null search to check if our position is still good enough
        self.search_stack.push_node(&nm_child);
        let reduced_child_depth = depth.saturating_sub(reduction + 1);
        let (score, _) =
            self.search_node(&nm_child, reduced_child_depth, ply + 1, null_bounds, false);
        self.search_stack.pop();

        // If opponent couldn't beat beta even with a free move, position is strong enough to prune
        if -score >= bounds.beta {
            self.tt.store(
                node.hash(),
                ply,
                depth.saturating_sub(reduction),
                bounds.beta,
                None,
                bounds.alpha,
                bounds.beta,
                None,
            );
            return Some(bounds.beta);
        }

        None
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
        if !can_reverse_futility_prune(
            depth,
            in_check,
            node.node_type(),
            self.config.rfp_max_depth.value,
        ) {
            return None;
        }

        let margin = rfp_margin(
            depth,
            self.config.rfp_base_margin.value,
            self.config.rfp_depth_multiplier.value,
            is_improving,
            self.config.rfp_improving_bonus.value,
        );
        if static_eval - margin >= bounds.beta && static_eval.abs() < RAZOR_NEAR_MATE {
            self.tt.store(
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
}
