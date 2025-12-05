use cozy_chess::{Board, Move, Piece};
use utils::{game_phase, is_capture};

use crate::{
    pruning::{
        can_futility_prune, can_null_move_prune, can_razor_prune, can_reverse_futility_prune,
        futility_margin, null_move_reduction, razor_margin, rfp_margin, RAZOR_NEAR_MATE,
    },
    stack::SearchNode,
    utils::see::see,
};

use super::Engine;

impl Engine {
    /// Futility pruning: skip moves unlikely to raise alpha based on static eval + margin.
    ///
    /// <https://www.chessprogramming.org/Futility_Pruning>
    pub(super) fn try_futility_prune(
        &self,
        remaining_depth: u8,
        in_check: bool,
        is_tactical: bool,
        alpha: i16,
        static_eval: i16,
    ) -> bool {
        if !can_futility_prune(
            remaining_depth,
            in_check,
            self.config.futility_max_depth.value,
        ) {
            return false;
        }
        let margin = futility_margin(
            remaining_depth,
            self.config.futility_base_margin.value,
            self.config.futility_depth_multiplier.value,
        );
        !is_tactical && static_eval + margin <= alpha
    }

    /// Razoring: if eval is far below alpha, drop into qsearch to verify and return early.
    ///
    /// <https://www.chessprogramming.org/Razoring>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_razor_prune(
        &mut self,
        board: &Board,
        remaining_depth: u8,
        alpha: i16,
        depth: u8,
        in_check: bool,
        static_eval: i16,
    ) -> Option<i16> {
        if !can_razor_prune(remaining_depth, in_check, self.config.razor_max_depth.value) {
            return None;
        }
        // If static eval already near/above alpha threshold, do not razor
        let margin = razor_margin(
            remaining_depth,
            self.config.razor_base_margin.value,
            self.config.razor_depth_coefficient.value,
        );
        if static_eval >= alpha - margin {
            return None;
        }
        // Q search with null window
        let (value, _) = self.quiescence_search(board, alpha - 1, alpha, depth);
        if value < alpha && value.abs() < RAZOR_NEAR_MATE {
            Some(value)
        } else {
            None
        }
    }

    /// SEE pruning: skip bad captures based on static exchange evaluation.
    /// Only runs expensive SEE on questionable captures (victim < attacker), using a dynamic
    /// threshold based on depth and eval gap. Inspired by Black Marlin.
    ///
    /// <https://www.chessprogramming.org/Static_Exchange_Evaluation>
    /// <https://github.com/jnlt3/blackmarlin>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_see_prune(
        &self,
        board: &Board,
        m: Move,
        moved_piece: Piece,
        remaining_depth: u8,
        in_check: bool,
        is_pv_node: bool,
        is_pv_move: bool,
        alpha: i16,
        static_eval: i16,
    ) -> bool {
        if in_check
            || is_pv_node
            || is_pv_move
            || remaining_depth < self.config.see_prune_min_remaining_depth.value
            || remaining_depth > self.config.see_prune_max_depth.value
        {
            return false;
        }

        if !is_capture(board, m) {
            return false;
        }

        let captured_piece = board.piece_on(m.to).unwrap();

        // Promotion capture is likely good
        if m.promotion.is_some() {
            return false;
        }

        let phase = game_phase(board);
        let captured_value = self.config.get_piece_values().get(captured_piece, phase);
        let attacker_value = self.config.get_piece_values().get(moved_piece, phase);

        // Only run SEE on questionable captures (expensive):
        // Skip if: victim >= attacker (looks good)
        if captured_value >= attacker_value {
            return false;
        }
        // OR if attacker is not worth checking SEE for
        if attacker_value < self.config.see_prune_min_attacker_value.value {
            return false;
        }

        // Dynamic SEE threshold: how much material loss is acceptable?
        // - eval_gap: if we're far below alpha, we need the capture to work out
        // - depth_margin: at higher depths, be more conservative (less pruning)
        // A negative threshold means we can afford to lose some material
        let eval_gap = alpha - static_eval;
        let depth_margin = self.config.see_prune_depth_margin.value * (remaining_depth as i16);
        let see_threshold = -(eval_gap.max(0) + depth_margin);

        !see(
            board,
            m,
            phase,
            &self.config.get_piece_values(),
            see_threshold,
        )
    }

    /// Null move pruning: give opponent a free move; if we still beat beta, prune the subtree.
    /// Includes verification search at low depths to avoid zugzwang.
    ///
    /// <https://www.chessprogramming.org/Null_Move_Pruning>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_null_move_prune(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        hash: u64,
        remaining_depth: u8,
        in_check: bool,
        try_null_move: bool,
        static_eval: Option<i16>,
    ) -> Option<i16> {
        if !(try_null_move
            && can_null_move_prune(
                board,
                remaining_depth,
                in_check,
                self.config.nmp_min_depth.value,
            ))
        {
            return None;
        }
        let nm_board = board.null_move()?;
        let base_remaining = max_depth - depth;

        // Calculate reduction based on remaining depth and static eval
        let r = null_move_reduction(
            base_remaining,
            static_eval,
            beta,
            self.config.nmp_base_reduction.value,
            self.config.nmp_depth_divisor.value,
            self.config.nmp_eval_margin.value,
        );

        // Do a reduced depth null search to check if our position is still good enough
        self.search_stack.push(SearchNode::new(nm_board.hash()));
        let (score, _) = self.search_subtree(
            &nm_board,
            depth + 1,
            max_depth - r,
            -beta,
            -beta + 1,
            false,
            false,
        );
        self.search_stack.pop();

        // If opponent couldn't beat beta even with a free move, position is strong enough to prune
        if -score >= beta {
            // Zugzwang check: at shallow depths, verify with a real search.
            // In zugzwang, passing is better than any legal move, so null move gives false positive.
            if base_remaining <= 6 {
                self.search_stack.push(SearchNode::new(nm_board.hash()));
                let verify_depth = max_depth - r.saturating_sub(1);
                let (v_score, _) = self.search_subtree(
                    &nm_board,
                    depth + 1,
                    verify_depth,
                    -beta,
                    -beta + 1,
                    false,
                    false,
                );
                self.search_stack.pop();
                if -v_score < beta {
                    return None; // fail verification; do not prune
                }
            }

            let null_move_depth = max_depth - r;
            self.tt
                .store(hash, depth, null_move_depth, beta, None, alpha, beta, None);
            return Some(beta);
        }

        None
    }

    /// Reverse futility pruning: if static eval - margin >= beta, the position is too good to search.
    ///
    /// <https://www.chessprogramming.org/Reverse_Futility_Pruning>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_reverse_futility_prune(
        &mut self,
        remaining_depth: u8,
        in_check: bool,
        is_pv_node: bool,
        static_eval: i16,
        beta: i16,
        hash: u64,
        depth: u8,
        _max_depth: u8,
        alpha: i16,
        is_improving: bool,
    ) -> Option<i16> {
        if !can_reverse_futility_prune(
            remaining_depth,
            in_check,
            is_pv_node,
            self.config.rfp_max_depth.value,
        ) {
            return None;
        }

        let margin = rfp_margin(
            remaining_depth,
            self.config.rfp_base_margin.value,
            self.config.rfp_depth_multiplier.value,
            is_improving,
            self.config.rfp_improving_bonus.value,
        );
        if static_eval - margin >= beta && static_eval.abs() < RAZOR_NEAR_MATE {
            let rfp_depth = depth;
            self.tt.store(
                hash,
                depth,
                rfp_depth,
                beta,
                Some(static_eval),
                alpha,
                beta,
                None,
            );
            return Some(beta);
        }
        None
    }

    /// Internal Iterative Deepening: do a shallow search to get a best move for ordering when TT misses.
    ///
    /// <https://www.chessprogramming.org/Internal_Iterative_Deepening>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_iid(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        try_null_move: bool,
        allow_iid: bool,
        need_iid: bool,
        remaining_depth: u8,
        in_check: bool,
    ) -> Option<Move> {
        if !(allow_iid && need_iid && remaining_depth >= 4 && !in_check) {
            return None;
        }
        let shallow_max = max_depth.saturating_sub(self.config.iid_reduction.value);
        let (.., shallow_line) = self.search_subtree(
            board,
            depth,
            shallow_max,
            alpha,
            beta,
            try_null_move,
            false, // disable nested IID
        );
        shallow_line.first().copied()
    }
}
