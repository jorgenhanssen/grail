use chess::{Board, ChessMove};

use crate::{
    pruning::{
        can_futility_prune, can_null_move_prune, can_razor_prune, can_reverse_futility_prune,
        futility_margin, null_move_reduction, razor_margin, rfp_margin, RAZOR_NEAR_MATE,
    },
    stack::SearchNode,
};

use super::Engine;

impl Engine {
    #[inline(always)]
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

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
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

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
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
        // Null move pruning: if giving the opponent a free move still doesn't let
        // them reach beta, the position is strong enough to prune

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
        self.search_stack.push(SearchNode::new(nm_board.get_hash()));
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

        // The opponent still can't reach beta,
        // so the position is strong enough to prune
        if -score >= beta {
            // However, in Zugzwang positions, passing is better than any legal move
            // so we need to verify that the position is still good enough
            if base_remaining <= 6 {
                self.search_stack.push(SearchNode::new(nm_board.get_hash()));
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

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
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

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
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
    ) -> Option<ChessMove> {
        // Gate
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
