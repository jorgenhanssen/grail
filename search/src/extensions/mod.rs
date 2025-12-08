use cozy_chess::{Board, Move, Piece};

mod passed_pawn;

pub fn get(board: &Board, m: &Move, moved_piece: Piece, is_capture: bool) -> u8 {
    passed_pawn::extension(board, m, moved_piece, is_capture)
}
