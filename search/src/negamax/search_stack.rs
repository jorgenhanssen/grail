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
    pub fn is_improving(&self, current_eval: i16) -> bool {
        const IMPROVING_MARGIN: i16 = 20;

        if self.nodes.len() >= 2 {
            let prev_eval = self.nodes[self.nodes.len() - 2].static_eval;
            if let Some(prev) = prev_eval {
                return current_eval > prev - IMPROVING_MARGIN;
            }
        }

        false
    }

    #[inline(always)]
    pub fn is_cycle(&self, hash: u64) -> bool {
        self.nodes.iter().filter(|n| n.hash == hash).count() > 1
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
