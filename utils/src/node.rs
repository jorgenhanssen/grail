use std::cell::OnceCell;

use cozy_chess::{BitBoard, Board, Color, GameStatus, Move, Piece, Square};

use crate::board_metrics::BoardMetrics;
use crate::is_zugzwang;
use crate::material::total_material;
use crate::moves::is_capture;

/// Alpha-beta node classification. Pv nodes return an exact score and have a
/// real PV; Cut nodes failed high and only need a lower bound; All nodes failed
/// low and only need an upper bound.
///
/// <https://www.chessprogramming.org/Node_Types>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Pv,
    Cut,
    All,
}

impl NodeType {
    pub fn is_pv(self) -> bool {
        matches!(self, Self::Pv)
    }

    pub fn is_cut(self) -> bool {
        matches!(self, Self::Cut)
    }

    /// First child of a Pv stays Pv, Cut/All swap. Everything past the first
    /// move is treated as a Cut since we're scout-searching with a null window.
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

    /// Flip the Cut/All expectation, used when handing a position to the
    /// opponent (null move, LMR re-search). Pv falls back to Cut since there's
    /// no meaningful inverse for it.
    pub fn inverted(self) -> Self {
        match self {
            Self::Pv => Self::Cut,
            Self::Cut => Self::All,
            Self::All => Self::Cut,
        }
    }
}

/// A node in the search tree. Wraps a board with its expected node type and a
/// lazily-computed metrics cache.
pub struct Node {
    board: Board,
    metrics: OnceCell<BoardMetrics>,
    node_type: NodeType,
}

impl Node {
    pub fn new(board: Board, node_type: NodeType) -> Self {
        Self {
            board,
            metrics: OnceCell::new(),
            node_type,
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn side_to_move(&self) -> Color {
        self.board.side_to_move()
    }

    pub fn hash(&self) -> u64 {
        self.board.hash()
    }

    pub fn in_check(&self) -> bool {
        !self.board.checkers().is_empty()
    }

    pub fn is_checkmate(&self) -> bool {
        self.in_check() && self.board.status() == GameStatus::Won
    }

    pub fn is_fifty_move_draw(&self) -> bool {
        self.board.halfmove_clock() >= 100 && !self.is_checkmate()
    }

    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        self.board.piece_on(sq)
    }

    pub fn is_capture(&self, mv: Move) -> bool {
        is_capture(&self.board, mv)
    }

    pub fn total_material(&self) -> i16 {
        total_material(&self.board)
    }

    pub fn is_zugzwang(&self) -> bool {
        is_zugzwang(&self.board)
    }

    pub fn colored_pieces(&self, color: Color, piece: Piece) -> BitBoard {
        self.board.colored_pieces(color, piece)
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    pub fn is_pv(&self) -> bool {
        self.node_type.is_pv()
    }

    pub fn is_cut(&self) -> bool {
        self.node_type.is_cut()
    }

    /// Change the node type without cloning the board.
    /// Used for re-searches where we need a different node type for the same position.
    pub fn set_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }

    pub fn metrics(&self) -> &BoardMetrics {
        self.metrics.get_or_init(|| BoardMetrics::new(&self.board))
    }

    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.metrics().threats[color as usize]
    }

    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.metrics().attacks[color as usize]
    }

    pub fn support_for(&self, color: Color) -> BitBoard {
        self.metrics().support[color as usize]
    }

    pub fn threats(&self) -> BitBoard {
        self.threats_for(self.side_to_move())
    }

    pub fn create_child(&self, mv: Move, mv_index: i32) -> Self {
        let mut board = self.board.clone();
        board.play_unchecked(mv);
        Self {
            board,
            metrics: OnceCell::new(),
            node_type: self.node_type.child(mv_index),
        }
    }

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
