use cozy_chess::{Move, Piece};

use utils::Node;

mod passed_pawn;

pub fn get(node: &Node, m: &Move, moved_piece: Piece, is_capture: bool) -> u8 {
    passed_pawn::extension(node, m, moved_piece, is_capture)
}
