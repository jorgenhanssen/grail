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

    /// Singular extension: extend search if TT move is clearly best.
    ///
    /// Based on Stockfish's restricted/modern singular extensions.
    /// <https://www.chessprogramming.org/Singular_Extensions>
    pub(super) fn get_singular_extension(
        &mut self,
        node: &Node,
        m: Move,
        tt: Option<ProbeResult>,
        depth: u8,
        ply: u8,
        is_singular_search: bool,
    ) -> u8 {
        // Need TT info with matching move
        let tt = match tt {
            Some(t) if t.best_move == Some(m) => t,
            _ => return 0,
        };

        // Don't nest singular searches
        if is_singular_search {
            return 0;
        }

        // Need sufficient depth
        if depth < self.config.singular_min_depth.value {
            return 0;
        }

        // TT entry must be deep enough (depth >= tt_depth - margin)
        if tt.depth + self.config.singular_depth_margin.value < depth {
            return 0;
        }

        // Only for lower bound or exact (move was good)
        if !matches!(tt.bound, Bound::Lower | Bound::Exact) {
            return 0;
        }

        // Skip mate scores
        if tt.value.abs() >= MATE_SCORE_BOUND {
            return 0;
        }

        let singular_beta =
            tt.value.saturating_sub((self.config.singular_beta_margin.value * depth as i16).max(1));
        let singular_depth = (depth - 1) / 2;

        // Reduced search excluding TT move
        self.search_stack
            .current_mut(|n| n.singular = Some(SingularSearch { excluded: m }));
        let (singular_value, _) =
            self.search_node(node, singular_depth, ply, Bounds::null(singular_beta - 1), false);
        self.search_stack.current_mut(|n| n.singular = None);

        if singular_value < singular_beta {
            // Double extend if very singular
            if singular_value < singular_beta.saturating_sub(self.config.double_ext_margin.value) {
                return 2;
            }
            return 1;
        }

        0
    }
}
