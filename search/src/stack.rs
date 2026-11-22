use cozy_chess::Move;
use utils::Node;

use crate::history::{PieceTo, PrevMoves};

/// Change in stack eval since last move (same side).
#[derive(Clone, Copy)]
pub struct EvalDelta(pub i32);

impl EvalDelta {
    pub fn is_improving(self) -> bool {
        const IMPROVING_MARGIN: i32 = 20;
        self.0 > -IMPROVING_MARGIN
    }
}

/// Context for singular extension search.
#[derive(Clone, Copy)]
pub struct SingularSearch {
    /// The TT move to exclude from this ply
    pub excluded: Move,
}

/// A node in the search stack, tracking state at each ply.
#[derive(Clone, Copy)]
pub struct SearchNode {
    /// Zobrist hash for repetition detection
    pub hash: u64,
    /// Move that led here, for continuation history
    pub moved: Option<PieceTo>,
    /// Best-known eval at this ply (TT score when available, else corrected static eval)
    pub eval: Option<i16>,
    /// Singular extension context (if in singular search)
    pub singular: Option<SingularSearch>,
}

impl SearchNode {
    pub fn new(hash: u64) -> Self {
        Self {
            hash,
            moved: None,
            eval: None,
            singular: None,
        }
    }

    pub fn with_move(hash: u64, moved: PieceTo) -> Self {
        Self {
            hash,
            moved: Some(moved),
            eval: None,
            singular: None,
        }
    }
}

/// Stack tracking the search path from root to current position.
pub struct SearchStack {
    nodes: Vec<SearchNode>,
}

impl SearchStack {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn push(&mut self, node: SearchNode) {
        self.nodes.push(node);
    }
    pub fn push_node(&mut self, node: &Node) {
        self.push(SearchNode::new(node.hash()));
    }

    pub fn push_move(&mut self, node: &Node, moved: PieceTo) {
        self.push(SearchNode::with_move(node.hash(), moved));
    }

    pub fn pop(&mut self) -> Option<SearchNode> {
        self.nodes.pop()
    }

    pub fn current_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SearchNode),
    {
        if let Some(node) = self.nodes.last_mut() {
            f(node);
        }
    }

    pub fn current(&self) -> Option<&SearchNode> {
        self.nodes.last()
    }

    pub fn eval_delta(&self) -> Option<EvalDelta> {
        let [.., previous, _, current] = self.nodes.as_slice() else {
            return None;
        };

        Some(EvalDelta(
            i32::from(current.eval?) - i32::from(previous.eval?),
        ))
    }

    /// Detects a single repetition and treats it as a draw.
    /// We don't require threefold because the search tends to cycle once it finds a repetition.
    pub fn is_repetition(&self, node: &Node, game_history: &ahash::AHashSet<u64>) -> bool {
        let ply = self.nodes.len() - 1;
        let hash = node.hash();
        let halfmove_clock = node.board().halfmove_clock() as usize;

        // No need to look back further than the halfmove clock because
        // you can't undo those moves.
        // Also stepped by 2 because color is baked into the hash, so one ply back can never match anyways.
        let max_back = halfmove_clock.min(ply);
        for back in (2..=max_back).step_by(2) {
            if self.nodes[ply - back].hash == hash {
                return true;
            }
        }

        // Same here, only bother with the game history if the clock reaches past the root.
        halfmove_clock > ply && game_history.contains(&hash)
    }

    /// Previous-move context for continuation history/correction.
    /// Slot i holds the move made i plies ago (0 = opponent last move).
    pub fn prev_moves(&self) -> PrevMoves {
        let mut prev_moves: PrevMoves = Default::default();
        let len = self.nodes.len();
        for (i, slot) in prev_moves.iter_mut().enumerate().take(len) {
            *slot = self.nodes[len - 1 - i].moved;
        }
        prev_moves
    }
}
