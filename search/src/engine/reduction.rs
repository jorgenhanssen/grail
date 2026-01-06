use cozy_chess::Move;
use utils::{creates_threat, evades_threat, Node};

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
        parent: &Node,
        child: &Node,
        m: Move,
        max_depth: u8,
        lmr_table: &LmrTable,
    ) -> Reduction {
        if is_pv_move {
            return Reduction::Reduction(0);
        }

        let hist =
            self.history_heuristic
                .get(parent.side_to_move(), m.from, m.to, parent.threats());

        // Late move reductions - later moves in ordering are less likely to be best
        let mut reduction = lmr_table.get(remaining_depth, move_index);

        // Reduce more
        if parent.is_cut() {
            reduction = reduction.saturating_add(1);
        }
        if !is_improving {
            reduction = reduction.saturating_add(1);

            if !is_capture && hist < self.history_heuristic.reduction_threshold() {
                reduction = reduction.saturating_add(1);
            }
        }

        // Reduce less
        if reduction > 0 {
            if near_root(depth, remaining_depth) {
                reduction = reduction.saturating_sub(1);
            }
            if parent.is_pv() {
                reduction = reduction.saturating_sub(1);
            }
            if is_tactical || creates_threat(parent, child) || evades_threat(parent, child) {
                reduction = reduction.saturating_sub(1);
            }
            if self.killer_moves[depth as usize].contains(&Some(m)) {
                reduction = reduction.saturating_sub(1);
            }
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
