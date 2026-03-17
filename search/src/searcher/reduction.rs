use cozy_chess::Piece;
use utils::{FracPly, Node, creates_threat, evades_threat, piece_value};

use crate::utils::near_root;

use super::Searcher;

pub enum Reduction {
    Reduce(u8),
    Prune,
}

impl Searcher {
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
        captured: Option<Piece>,
        is_promotion: bool,
        move_index: i32,
        parent: &Node,
        child: &Node,
        hist: i16,
        cont_hist: i16,
    ) -> Reduction {
        if is_pv_move {
            return Reduction::Reduce(0);
        }

        let mut reduction = self.lmr.get(depth, move_index);

        // Reduce more
        if parent.is_cut() {
            reduction += FracPly(self.config.reduction_cut_node.value);
        }
        if !is_improving {
            reduction += FracPly(self.config.reduction_not_improving.value);
        }

        if let Some(victim) = captured {
            history_reduction(
                &mut reduction,
                piece_value(victim),
                self.config.reduction_capture_value_divisor.value,
            );
            history_reduction(
                &mut reduction,
                hist,
                self.config.reduction_capture_history_divisor.value,
            );
        } else {
            history_reduction(
                &mut reduction,
                hist,
                self.config.reduction_history_divisor.value,
            );
            history_reduction(
                &mut reduction,
                cont_hist,
                self.config.reduction_cont_hist_divisor.value,
            );
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
            if captured.is_some() || is_promotion {
                reduction -= FracPly(self.config.anti_reduction_tactical.value);
            }
            if creates_threat(parent, child) || evades_threat(parent, child) {
                reduction -= FracPly(self.config.anti_reduction_threat.value);
            }
        }

        let r = reduction.whole().min(depth.saturating_sub(2));

        // Prune when bad history if it would barely search anyway
        let reduced_depth = depth.saturating_sub(r);
        if captured.is_none()
            && !is_improving
            && hist < self.history_heuristic.prune_threshold()
            && reduced_depth <= 1
        {
            return Reduction::Prune;
        }

        Reduction::Reduce(r)
    }
}

fn history_reduction(reduction: &mut FracPly, score: i16, divisor: i32) {
    let delta = FracPly((score.abs() as i32 * FracPly::ONE as i32 / divisor) as u16);

    if score > 0 {
        *reduction -= delta
    } else {
        *reduction += delta
    }
}
