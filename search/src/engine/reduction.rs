use cozy_chess::Move;
use utils::{FracPly, Node, creates_threat, evades_threat};

use crate::{lmr::LmrTable, utils::near_root};

use super::Engine;

pub enum Reduction {
    Reduce(u8),
    Prune,
}

impl Engine {
    /// Late move reductions: reduce search depth for moves unlikely to be best.
    ///
    /// <https://www.chessprogramming.org/Late_Move_Reductions>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn get_reduction(
        &self,
        ply: u8,
        depth: u8,
        is_pv_move: bool,
        is_improving: bool,
        is_capture: bool,
        is_promotion: bool,
        move_index: i32,
        parent: &Node,
        child: &Node,
        m: Move,
        lmr_table: &LmrTable,
    ) -> Reduction {
        if is_pv_move {
            return Reduction::Reduce(0);
        }

        let hist = self
            .history_heuristic
            .get(parent.side_to_move(), m.from, m.to);

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
            if parent.in_check() || child.in_check() {
                reduction -= FracPly(self.config.anti_reduction_check.value);
            }
            if is_capture || is_promotion {
                reduction -= FracPly(self.config.anti_reduction_tactical.value);
            }
            if creates_threat(parent, child) || evades_threat(parent, child) {
                reduction -= FracPly(self.config.anti_reduction_threat.value);
            }
        }

        // Scale up reductions for all-nodes — every move is expected to fail low.
        // Based on Stockfish's approach, which applies stronger reduction near leaves.
        if parent.is_all() {
            reduction += reduction / (depth as u16);
        }

        let r = reduction.whole().min(depth.saturating_sub(2));

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
