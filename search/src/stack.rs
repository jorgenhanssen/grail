use chess::{ChessMove, Piece};

#[derive(Clone, Copy)]
pub struct SearchNode {
    pub hash: u64,
    pub last_move: Option<ChessMove>,
    pub piece: Option<Piece>,
    pub static_eval: Option<i16>,
}

impl SearchNode {
    #[inline(always)]
    pub fn new(hash: u64) -> Self {
        Self {
            hash,
            last_move: None,
            piece: None,
            static_eval: None,
        }
    }

    #[inline(always)]
    pub fn with_move(hash: u64, mv: ChessMove, piece: Piece) -> Self {
        Self {
            hash,
            last_move: Some(mv),
            piece: Some(piece),
            static_eval: None,
        }
    }
}

pub struct SearchStack {
    nodes: Vec<SearchNode>,
}

impl SearchStack {
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    #[inline(always)]
    pub fn push(&mut self, node: SearchNode) {
        self.nodes.push(node);
    }
    #[inline(always)]
    pub fn push_move(&mut self, hash: u64, mv: ChessMove, piece: Piece) {
        self.push(SearchNode::with_move(hash, mv, piece));
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<SearchNode> {
        self.nodes.pop()
    }

    #[inline(always)]
    pub fn current(&self) -> SearchNode {
        *self.nodes.last().unwrap()
    }

    #[inline(always)]
    pub fn current_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SearchNode),
    {
        if let Some(node) = self.nodes.last_mut() {
            f(node);
        }
    }

    #[inline(always)]
    pub fn is_improving(&self) -> bool {
        const IMPROVING_MARGIN: i16 = 20;

        let len = self.nodes.len();
        if len < 3 {
            return false;
        }

        if let Some(current_eval) = self.nodes[len - 1].static_eval {
            // Compare 2 plies back (same side to move)
            if let Some(prev_eval) = self.nodes[len - 3].static_eval {
                return current_eval > prev_eval - IMPROVING_MARGIN;
            }
        }

        false
    }

    #[inline(always)]
    pub fn is_repetition(&self, game_history: &ahash::AHashSet<u64>) -> bool {
        let current_hash = self.nodes[self.nodes.len() - 1].hash;

        // Check if this position was seen in the game before we started searching
        if game_history.contains(&current_hash) {
            return true;
        }

        // Check if this position appeared earlier in the current search tree
        // (skip the last node which is the current position)
        for node in self.nodes.iter().rev().skip(1) {
            if node.hash == current_hash {
                return true;
            }
        }

        false
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[SearchNode] {
        &self.nodes
    }

    #[inline(always)]
    pub fn piece_repetition_penalty(&self, base_penalty: i16) -> i16 {
        let stack_len = self.nodes.len();
        if stack_len < 2 {
            return 0;
        }

        let last_piece = match self.nodes[stack_len - 1].piece {
            Some(p) => p,
            None => return 0,
        };

        let consecutive_count = self.nodes[..stack_len - 1]
            .iter()
            .rev()
            .take_while(|node| node.piece == Some(last_piece))
            .count();

        if consecutive_count == 0 {
            return 0;
        }

        base_penalty * (1 << consecutive_count)
    }
}
