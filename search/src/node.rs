/// Classification of nodes in the alpha-beta search tree.
///
/// See: <https://www.chessprogramming.org/Node_Types>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// PV-node: score falls within the window, `alpha < s < beta`.
    ///
    /// All moves are searched and the returned value is exact (not a bound),
    /// propagating up to the root along with the principal variation.
    ///
    /// - Root and leftmost nodes are always PV-nodes
    /// - Searched with open window (`beta - alpha > 1`)
    /// - Siblings of a PV-node are expected Cut-nodes
    Pv,
    /// Cut-node: fail-high, `s >= beta`. Score is a lower bound.
    ///
    /// At least one move must be searched before a beta-cutoff can occur.
    ///
    /// - Searched with null window
    /// - Child of a Cut-node is an All-node
    /// - Aggressive pruning allowed
    Cut,
    /// All-node: fail-low, `s <= alpha`. Score is an upper bound.
    ///
    /// No move exceeds alpha, so every move must be searched.
    ///
    /// - Searched with null window
    /// - Children of an All-node are Cut-nodes
    All,
}

impl NodeType {
    /// Returns if this is a PV node (full window search).
    #[inline]
    pub fn is_pv(self) -> bool {
        matches!(self, Self::Pv)
    }

    /// Returns if this is an expected cut-node.
    #[inline]
    pub fn is_cut(self) -> bool {
        matches!(self, Self::Cut)
    }

    /// Determines the child node type based on move index.
    ///
    /// First move (index 0):
    /// - PV's first child is PV (continues the principal variation)
    /// - Cut's first child is All (if we expect to fail high, child expects fail low)
    /// - All's first child is Cut
    ///
    /// Later moves (index > 0): always Cut (scout search expects cutoff)
    #[inline]
    pub fn child(self, move_index: i32) -> Self {
        if move_index == 0 {
            match self {
                Self::Pv => Self::Pv,
                Self::Cut => Self::All,
                Self::All => Self::Cut,
            }
        } else {
            Self::Cut
        }
    }

    /// Inverts the Cut/All expectation.
    ///
    /// In alpha-beta, if we expect to fail high (Cut), our opponent expects
    /// to fail low (All), and vice versa.
    ///
    /// Used when passing to a child with opposite expectations:
    /// - Null move: we expect fail-high, so opponent should fail-low even with extra tempo
    /// - LMR re-search: a surprising result needs verification with flipped expectation
    #[inline]
    pub fn inverted(self) -> Self {
        match self {
            // PV shouldn't be inverted, but default to Cut for scout-like searches
            Self::Pv => Self::Cut,
            // We expect fail-high, so opponent expects fail-low (can't refute our advantage)
            Self::Cut => Self::All,
            // We expect fail-low, so opponent expects to cut (can exploit our weakness)
            Self::All => Self::Cut,
        }
    }
}
