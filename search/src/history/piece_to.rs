use cozy_chess::{Color, Piece, Square};

/// A colored piece and its destination square, used as a history table key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieceTo {
    pub color: Color,
    pub piece: Piece,
    pub to: Square,
}

impl PieceTo {
    /// Number of distinct PieceTo values: 2 colors * 6 pieces * 64 squares = 768.
    pub const SIZE: usize = Color::NUM * Piece::NUM * Square::NUM;

    /// Create a new PieceTo value.
    pub const fn new(color: Color, piece: Piece, to: Square) -> Self {
        Self { color, piece, to }
    }

    /// Flattened index (0-767): (color * 6 + piece) * 64 + square
    pub const fn index(&self) -> usize {
        (self.color as usize * Piece::NUM + self.piece as usize) * Square::NUM + self.to as usize
    }
}
