use chess::{BitBoard, Board, ChessMove};

use crate::reductions::lmr;

use super::Engine;

impl Engine {
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn get_reduction(
        &mut self,
        board: &Board,
        mv: ChessMove,
        depth: u8,
        max_depth: u8,
        remaining_depth: u8,
        in_check: bool,
        is_tactical: bool,
        move_index: i32,
        is_pv_move: bool,
        is_improving: bool,
        pre_move_threats: BitBoard,
    ) -> (u8, bool) {
        let mut reduction = lmr(
            remaining_depth,
            is_tactical,
            move_index,
            is_pv_move,
            is_improving,
            self.config.lmr_min_depth.value,
            self.config.lmr_divisor.value as f32 / 100.0,
            self.config.lmr_max_reduction_ratio.value as f32 / 100.0,
        );

        // Apply history-based pruning/reduction adjustment
        let should_prune = self.history_heuristic.maybe_reduce_or_prune(
            board,
            mv,
            depth,
            max_depth,
            remaining_depth,
            in_check,
            is_tactical,
            is_pv_move,
            move_index,
            is_improving,
            &mut reduction,
            pre_move_threats,
        );

        (reduction, should_prune)
    }
}
