use std::cell::OnceCell;

use cozy_chess::{BitBoard, Board, Color, GameStatus, Move, Piece, Square};

use crate::board_metrics::BoardMetrics;
use crate::is_zugzwang;
use crate::material::total_material;
use crate::moves::is_capture;

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
    pub fn is_pv(self) -> bool {
        matches!(self, Self::Pv)
    }

    /// Returns if this is an expected cut-node.
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

/// A node in the game tree.
pub struct Node {
    /// The board state at this node.
    board: Board,
    /// Cached board metrics (attacks, threats, support), computed lazily.
    metrics: OnceCell<BoardMetrics>,
    /// The expected node type (PV, Cut, All) for search heuristics.
    node_type: NodeType,
}

impl Node {
    /// Create a new node
    pub fn new(board: Board, node_type: NodeType) -> Self {
        Self {
            board,
            metrics: OnceCell::new(),
            node_type,
        }
    }

    /// Get a reference to the board.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Get the side to move.
    pub fn side_to_move(&self) -> Color {
        self.board.side_to_move()
    }

    /// Get the board hash.
    pub fn hash(&self) -> u64 {
        self.board.hash()
    }

    /// Check if the side to move is in check.
    pub fn in_check(&self) -> bool {
        !self.board.checkers().is_empty()
    }

    /// Check if the position is checkmate.
    pub fn is_checkmate(&self) -> bool {
        self.in_check() && self.board.status() == GameStatus::Won
    }

    /// Check if the 50-move rule has been exceeded (100+ half-moves without pawn move or capture).
    pub fn is_fifty_move_draw(&self) -> bool {
        self.board.halfmove_clock() >= 100 && !self.is_checkmate()
    }

    /// Get the piece on a square.
    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        self.board.piece_on(sq)
    }

    /// Check if a move is a capture.
    pub fn is_capture(&self, mv: Move) -> bool {
        is_capture(&self.board, mv)
    }

    /// Get total material on the board.
    pub fn total_material(&self) -> i16 {
        total_material(&self.board)
    }

    /// Check if the position is zugzwang.
    pub fn is_zugzwang(&self) -> bool {
        is_zugzwang(&self.board)
    }

    /// Get pieces of a specific color and type.
    pub fn colored_pieces(&self, color: Color, piece: Piece) -> BitBoard {
        self.board.colored_pieces(color, piece)
    }

    /// Get the node type.
    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Check if this is a PV node.
    pub fn is_pv(&self) -> bool {
        self.node_type.is_pv()
    }

    /// Check if this is a Cut node.
    pub fn is_cut(&self) -> bool {
        self.node_type.is_cut()
    }

    /// Change the node type without cloning the board.
    /// Used for re-searches where we need a different node type for the same position.
    pub fn set_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }

    /// Get or compute the board metrics.
    pub fn metrics(&self) -> &BoardMetrics {
        self.metrics.get_or_init(|| BoardMetrics::new(&self.board))
    }

    /// Get the threats bitboard for a color (opponent's valuable pieces under attack).
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.metrics().threats[color as usize]
    }

    /// Get the attack bitboard for a color.
    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.metrics().attacks[color as usize]
    }

    /// Get the support bitboard for a color (own pieces defended by own pieces).
    pub fn support_for(&self, color: Color) -> BitBoard {
        self.metrics().support[color as usize]
    }

    /// Get threats for the side to move (convenience method).
    pub fn threats(&self) -> BitBoard {
        self.threats_for(self.side_to_move())
    }

    /// Create a child node by playing a move.
    pub fn create_child(&self, mv: Move, mv_index: i32) -> Self {
        let mut board = self.board.clone();
        board.play_unchecked(mv);
        Self {
            board,
            metrics: OnceCell::new(),
            node_type: self.node_type.child(mv_index),
        }
    }

    /// Create a null-move child (pass turn to opponent).
    pub fn create_null_move_child(&self) -> Option<Self> {
        self.board.null_move().map(|board| Self {
            board,
            metrics: OnceCell::new(),
            node_type: self.node_type.inverted(),
        })
    }
}

/// Returns true if the move creates new threats to opponent pieces.
pub fn creates_threat(parent: &Node, child: &Node) -> bool {
    let them = !parent.side_to_move();
    child.threats_for(them).len() > parent.threats_for(them).len()
}

/// Returns true if the move removes threats from our pieces.
pub fn evades_threat(parent: &Node, child: &Node) -> bool {
    let us = parent.side_to_move();
    child.threats_for(us).len() < parent.threats_for(us).len()
}
