#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod attacks;
pub mod bitset;
pub mod board_metrics;
mod eval;
mod material;
mod math;
pub mod memory;
mod moves;
mod node;
mod zobrist;

pub use attacks::{get_attackers_to, get_discovered_attacks};
pub use eval::flip_eval_perspective;
pub use material::{
    cap_eval_by_material, game_phase, has_insufficient_material, is_zugzwang, majors, minors,
    piece_value, side_has_insufficient_material, total_material, BISHOP_VALUE, KNIGHT_VALUE,
    PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE,
};
pub use math::select_softmax;
pub use moves::{
    captured_piece, collect_legal_moves, gives_check, has_check, has_legal_moves, is_capture,
    is_en_passant, make_move, only_move,
};
pub use node::{creates_threat, evades_threat, Node, NodeType};
pub use zobrist::pawn_key;
