use cozy_chess::{BitBoard, Move};
use utils::Node;

use crate::{
    reductions::{cap_reduction, LmrTable},
    utils::near_root,
};

use super::Engine;

pub enum Reduction {
    Reduction(u8),
    Prune,
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn get_reduction(
        &self,
        depth: u8,
        remaining_depth: u8,
        is_pv_move: bool,
        is_tactical: bool,
        is_improving: bool,
        is_capture: bool,
        move_index: i32,
        node: &Node,
        m: Move,
        max_depth: u8,
        pre_threats: BitBoard,
        new_threats: BitBoard,
        lmr_table: &LmrTable,
    ) -> Reduction {
        if is_pv_move {
            return Reduction::Reduction(0);
        }

        let hist = self
            .history_heuristic
            .get(node.side_to_move(), m.from, m.to, pre_threats);

        // Late move reductions - worse-sorted moves get reduced more
        let mut reduction = lmr_table.get(remaining_depth, move_index);

        // Reduce more
        if node.is_cut() {
            reduction = reduction.saturating_add(1);
        }
        if !is_improving {
            reduction = reduction.saturating_add(1);

            if !is_capture && hist < self.history_heuristic.reduction_threshold() {
                reduction = reduction.saturating_add(1);
            }
        }

        // Reduce less
        if near_root(depth, remaining_depth) {
            reduction = reduction.saturating_sub(1);
        }
        if is_tactical {
            reduction = reduction.saturating_sub(1);
        }
        if node.is_pv() {
            reduction = reduction.saturating_sub(1);
        }
        if new_threats.len() > pre_threats.len() {
            reduction = reduction.saturating_sub(1);
        }
        if self.killer_moves[depth as usize].contains(&Some(m)) {
            reduction = reduction.saturating_sub(1);
        }

        reduction = cap_reduction(reduction, remaining_depth);

        // Prune when bad history if it would barely search anyway
        let reduced_depth = max_depth.saturating_sub(reduction);
        if !is_capture
            && !is_improving
            && hist < self.history_heuristic.prune_threshold()
            && reduced_depth <= depth + 1
        {
            return Reduction::Prune;
        }

        Reduction::Reduction(reduction)
    }
}
