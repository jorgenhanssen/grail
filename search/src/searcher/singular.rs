use crate::scores::MATE_SCORE_BOUND;
use cozy_chess::Move;
use utils::Node;

use crate::{
    stack::SingularSearch,
    transposition::{Bound, ProbeResult},
    utils::Bounds,
};

use super::Searcher;

/// Result of probing for singular extension or multi-cut.
#[derive(Default)]
pub(super) struct SingularProbeResult {
    /// Extension to apply to the TT move (can be negative).
    pub extension: i8,
    /// If Some, prune the node and return this value.
    pub multi_cut: Option<i16>,
}

impl Searcher {
    /// Singular probe: evaluate TT move for extension or multi-cut prune.
    ///
    /// Based on Stockfish's restricted/modern singular extension logic.
    /// <https://www.chessprogramming.org/Singular_Extensions>
    /// <https://www.chessprogramming.org/Multi-Cut>
    #[allow(clippy::too_many_arguments)]
    pub(super) fn probe_singular(
        &mut self,
        node: &Node,
        m: Move,
        tt: Option<ProbeResult>,
        depth: u8,
        ply: u8,
        singular_active: bool,
        beta: i16,
    ) -> SingularProbeResult {
        let mut result = SingularProbeResult::default();

        // Don't at root or if already in singular search
        if ply == 0 || singular_active {
            return result;
        }

        // Only probe a matching TT move with usable data.
        let tt = match tt {
            Some(t) => t,
            None => return result,
        };
        if !is_sufficient_singular_tt_entry(
            tt,
            m,
            depth,
            self.config.singular_min_depth.value,
            self.config.singular_depth_margin.value,
        ) {
            return result;
        }

        let singular_depth = (depth - 1) / 2;
        let singular_beta = tt.value.saturating_sub(
            (self.config.singular_beta_margin.value as i32 * depth as i32 / 100).max(1) as i16,
        );

        // Reduced null-window search excluding TT move.
        self.search_stack
            .current_mut(|n| n.singular = Some(SingularSearch { excluded: m }));
        let singular_value = self.search_node(
            node,
            singular_depth,
            ply,
            Bounds::null(singular_beta - 1),
            false,
        );
        self.search_stack.current_mut(|n| n.singular = None);

        if singular_value < singular_beta {
            // TT move is uniquely strong: extend (extra if very singular).
            // Widen the margin when ply exceeds root depth to make double
            // extensions progressively harder deep in the tree.
            // Should help cases where TT hits givve check chains (KQ+K,,, etc).
            let overshoot = ply.saturating_sub(self.root_depth) as i16;
            let double_margin = self.config.double_ext_margin.value
                + overshoot * overshoot * self.config.double_ext_overshoot_penalty.value;
            if singular_value < singular_beta.saturating_sub(double_margin) {
                result.extension = 2;
                return result;
            }
            result.extension = 1;
            return result;
        }

        // Multi-cut pruning: if singular_beta already exceeds beta, multiple
        // moves are likely good enough to cause a cutoff and we can prune.
        // <https://www.chessprogramming.org/Multi-Cut>
        if singular_value >= beta && !node.is_pv() && singular_value.abs() < MATE_SCORE_BOUND {
            result.multi_cut = Some(singular_value);
            return result;
        }

        // Negative extensions (based on Stockfish).
        // <https://www.chessprogramming.org/Extensions>
        if tt.value >= beta {
            result.extension = -self.config.negative_ext_tt_high.value;
        } else if node.is_cut() {
            result.extension = -self.config.negative_ext_cut_node.value;
        }

        result
    }
}

fn is_sufficient_singular_tt_entry(
    tt: ProbeResult,
    m: Move,
    depth: u8,
    min_depth: u8,
    depth_margin: u8,
) -> bool {
    tt.best_move == Some(m)
        && depth >= min_depth
        && tt.depth + depth_margin >= depth
        && matches!(tt.bound, Bound::Lower | Bound::Exact)
        && tt.value.abs() < MATE_SCORE_BOUND
}
