use cozy_chess::{Color, Move, Piece, Rank};
use utils::Node;

use super::Searcher;

impl Searcher {
    /// Non-singular extensions: passed pawn pushes to 7th rank, etc.
    /// Single entry point keeps the search loop lean.
    pub(super) fn get_extension(
        &self,
        node: &Node,
        m: &Move,
        moved_piece: Piece,
        is_capture: bool,
    ) -> u8 {
        // Passed pawn extension: extend non-capture pawn pushes to 7th rank.
        // https://www.chessprogramming.org/Passed_Pawn_Extensions
        if moved_piece == Piece::Pawn && !is_capture && m.promotion.is_none() {
            let is_seventh = match node.side_to_move() {
                Color::White => m.to.rank() == Rank::Seventh,
                Color::Black => m.to.rank() == Rank::Second,
            };
            if is_seventh {
                return 1;
            }
        }

        0
    }
}
