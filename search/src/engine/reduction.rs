use cozy_chess::Move;
use utils::{creates_threat, evades_threat, FracPly, Node};

use crate::{
    reductions::{cap_reduction, LmrTable},
    utils::near_root,
};

use super::Engine;

pub enum Reduction {
    Reduce(u8),
    Prune,
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn get_reduction(
        &self,
        ply: u8,
        depth: u8,
        is_pv_move: bool,
        is_tactical: bool,
        is_improving: bool,
        is_capture: bool,
        move_index: i32,
        parent: &Node,
        child: &Node,
        m: Move,
        lmr_table: &LmrTable,
    ) -> Reduction {
        if is_pv_move {
            return Reduction::Reduce(0);
        }
        if is_tactical && is_improving {
            return Reduction::Reduce(0);
        }

        let hist =
            self.history_heuristic
                .get(parent.side_to_move(), m.from, m.to, parent.threats());

        let mut reduction = lmr_table.get(depth, move_index);

        // Reduce more
        if parent.is_cut() {
            reduction += FracPly(self.config.reduction_cut_node.value);
        }
        if !is_improving {
            reduction += FracPly(self.config.reduction_not_improving.value);

            if !is_capture && hist < self.history_heuristic.reduction_threshold() {
                reduction += FracPly(self.config.reduction_bad_history.value);
            }
        }

        // Reduce less
        if reduction > FracPly(0) {
            if near_root(ply, depth) {
                reduction -= FracPly(self.config.anti_reduction_near_root.value);
            }
            if parent.is_pv() {
                reduction -= FracPly(self.config.anti_reduction_pv_node.value);
            }
            if is_tactical || creates_threat(parent, child) || evades_threat(parent, child) {
                reduction -= FracPly(self.config.anti_reduction_tactical.value);
            }
        }

        let r = cap_reduction(reduction.whole(), depth);

        // Prune when bad history if it would barely search anyway
        let reduced_depth = depth.saturating_sub(r);
        if !is_capture
            && !is_improving
            && hist < self.history_heuristic.prune_threshold()
            && reduced_depth <= 1
        {
            return Reduction::Prune;
        }

        Reduction::Reduce(r)
    }
}
