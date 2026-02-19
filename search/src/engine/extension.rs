use cozy_chess::{Move, Piece};
use utils::Node;

use crate::extensions::passed_pawn;

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
}
