use utils::{FracPly, Node, creates_threat, evades_threat};

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
        tt_pv: bool,
        tt_age: u8,
    ) -> u8 {
        let mut reduction = self.lmr.get(depth, move_index);

        // Reduce more
        if parent.is_cut() {
            reduction += FracPly(self.config.reduction_cut_node.value);
        }
        if !is_improving {
            reduction += FracPly(self.config.reduction_not_improving.value);
        }
        if tt_move_is_capture && !is_capture {
            reduction += FracPly(self.config.reduction_quiets_if_tt_capture.value);
        }

        let hist_divisor = if is_capture {
            self.config.reduction_capture_history_divisor.value
        } else {
            self.config.reduction_history_divisor.value
        };

        history_reduction(&mut reduction, hist, hist_divisor);
        history_reduction(
            &mut reduction,
            cont_hist,
            self.config.reduction_cont_hist_divisor.value,
        );

        // Reduce less
        if reduction > FracPly(0) {
            if near_root(ply, depth) {
                reduction -= FracPly(self.config.anti_reduction_near_root.value);
            }
            if is_pv_move {
                reduction -= FracPly(self.config.anti_reduction_pv_move.value);
            }
            if parent.is_pv() {
                reduction -= FracPly(self.config.anti_reduction_pv_node.value);
            } else if tt_pv {
                // Node that has been a PV earlier (tapers off as the TT entry goes stale)
                let base = self.config.anti_reduction_tt_pv.value as u32;
                let falloff = tt_age as u32 * self.config.anti_reduction_tt_pv_decay.value as u32;
                reduction -= FracPly(base.saturating_sub(falloff) as u16);
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

        reduction.whole().min(depth.saturating_sub(2))
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

pub fn near_root(ply: u8, depth: u8) -> bool {
    let total_depth = depth + ply;
    ply <= total_depth >> 3 // 12.5% of the total depth
}
