use cozy_chess::{Color, Move, Piece};
use utils::Node;

use crate::history::{PieceTo, PrevMoves};

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
    /// Move that led to this position
    pub last_move: Option<Move>,
    /// Piece that moved (for continuation history)
    pub piece: Option<Piece>,
    /// Color of the piece that moved (for continuation history)
    pub color: Option<Color>,
    /// Best-known eval at this ply (TT score when available, else corrected static eval)
    pub eval: Option<i16>,
    /// Singular extension context (if in singular search)
    pub singular: Option<SingularSearch>,
}

impl SearchNode {
    pub fn new(hash: u64) -> Self {
        Self {
            hash,
            last_move: None,
            piece: None,
            color: None,
            eval: None,
            singular: None,
        }
    }

    pub fn with_move(hash: u64, mv: Move, piece: Piece, color: Color) -> Self {
        Self {
            hash,
            last_move: Some(mv),
            piece: Some(piece),
            color: Some(color),
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

    pub fn push_move(&mut self, node: &Node, mv: Move, piece: Piece, color: Color) {
        self.push(SearchNode::with_move(node.hash(), mv, piece, color));
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

    /// Returns true if eval improved vs 2 plies ago (same side to move).
    pub fn is_improving(&self) -> bool {
        const IMPROVING_MARGIN: i16 = 20;

        let len = self.nodes.len();
        if len < 3 {
            return false;
        }

        if let Some(current_eval) = self.nodes[len - 1].eval {
            if let Some(prev_eval) = self.nodes[len - 3].eval {
                return current_eval > prev_eval - IMPROVING_MARGIN;
            }
        }

        false
    }

    /// Detects a single repetition and treats it as a draw.
    /// We don't require threefold because the search tends to cycle once it finds a repetition.
    pub fn is_repetition(&self, game_history: &ahash::AHashSet<u64>) -> bool {
        let current_hash = self.nodes[self.nodes.len() - 1].hash;

        // Check if this position was seen in the game before we started searching
        if game_history.contains(&current_hash) {
            return true;
        }

        // Check search path (skip current position)
        for node in self.nodes.iter().rev().skip(1) {
            if node.hash == current_hash {
                return true;
            }
        }

        false
    }

    /// Previous-move context for continuation history/correction.
    /// Slot i holds the move made i plies ago (0 = opponent last move).
    pub fn prev_moves(&self) -> PrevMoves {
        let mut prev_moves: PrevMoves = Default::default();
        let len = self.nodes.len();
        for (i, slot) in prev_moves.iter_mut().enumerate().take(len) {
            let node = &self.nodes[len - 1 - i];
            if let (Some(mv), Some(piece), Some(color)) = (node.last_move, node.piece, node.color) {
                *slot = Some(PieceTo::new(color, piece, mv.to));
            }
        }
        prev_moves
    }
}
