use utils::{FracPly, Node, creates_threat, evades_threat};

use crate::utils::near_root;

use super::Searcher;

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
        is_capture: bool,
        is_promotion: bool,
        move_index: i32,
        parent: &Node,
        child: &Node,
        hist: i16,
        cont_hist: i16,
        tt_move_is_capture: bool,
    ) -> u8 {
        let mut reduction = self.lmr.get(depth, move_index);

        // Reduce more
        if parent.is_cut() {
            reduction += FracPly(self.config.reduction_cut_node);
        }
        if !is_improving {
            reduction += FracPly(self.config.reduction_not_improving);
        }
        if tt_move_is_capture && !is_capture {
            reduction += FracPly(self.config.reduction_quiets_if_tt_capture);
        }

        let hist_divisor = if is_capture {
            self.config.reduction_capture_history_divisor
        } else {
            self.config.reduction_history_divisor
        };

        reduction -= FracPly(hist as i32 * FracPly::ONE / hist_divisor);
        reduction -=
            FracPly(cont_hist as i32 * FracPly::ONE / self.config.reduction_cont_hist_divisor);

        // Reduce less
        if reduction > FracPly(0) {
            if is_pv_move {
                reduction -= FracPly(self.config.anti_reduction_pv_move);
            }
            if near_root(ply, depth) {
                reduction -= FracPly(self.config.anti_reduction_near_root);
            }
            if parent.is_pv() {
                reduction -= FracPly(self.config.anti_reduction_pv_node);
            }
            if parent.in_check() || child.in_check() {
                reduction -= FracPly(self.config.anti_reduction_check);
            }
            if is_capture || is_promotion {
                reduction -= FracPly(self.config.anti_reduction_tactical);
            }
            if creates_threat(parent, child) || evades_threat(parent, child) {
                reduction -= FracPly(self.config.anti_reduction_threat);
            }
        }

        reduction.whole().clamp(0, depth.saturating_sub(2) as i32) as u8
    }
}
