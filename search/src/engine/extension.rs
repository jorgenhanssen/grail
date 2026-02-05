use cozy_chess::{Move, Piece};
use utils::Node;

use crate::{
    extensions::passed_pawn,
    pruning::MATE_SCORE_BOUND,
    stack::SingularSearch,
    transposition::{Bound, ProbeResult},
    utils::Bounds,
};

use super::Engine;

#[derive(Default)]
pub(super) struct SingularProbeResult {
    pub extension: u8,
    pub multi_cut: Option<i16>,
}

impl Engine {
    pub(super) fn get_extension(
        &self,
        node: &Node,
        m: &Move,
        moved_piece: Piece,
        is_capture: bool,
    ) -> u8 {
        passed_pawn::extension(node, m, moved_piece, is_capture)
    }

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

        // Don't nest singular searches
        if singular_active {
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
        let singular_beta = tt
            .value
            .saturating_sub((self.config.singular_beta_margin.value * depth as i16).max(1));

        // Reduced null-window search excluding TT move.
        self.search_stack
            .current_mut(|n| n.singular = Some(SingularSearch { excluded: m }));
        let (singular_value, _) = self.search_node(
            node,
            singular_depth,
            ply,
            Bounds::null(singular_beta - 1),
            false,
        );
        self.search_stack.current_mut(|n| n.singular = None);

        if singular_value < singular_beta {
            // TT move is uniquely strong: extend (double if very singular).
            if singular_value < singular_beta.saturating_sub(self.config.double_ext_margin.value) {
                result.extension = 2;
                return result;
            }
            result.extension = 1;
            return result;
        }

        // If the reduced search fails high even without the TT/best move,
        // the position is so good that another move also beats beta, so we can prune.
        // <https://www.chessprogramming.org/Multi-Cut>
        if !node.is_pv()
            && beta.abs() < MATE_SCORE_BOUND
            && singular_value >= beta
            && singular_value.abs() < MATE_SCORE_BOUND
        {
            result.multi_cut = Some(singular_value);
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
