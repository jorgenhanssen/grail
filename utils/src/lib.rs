#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod attacks;
pub mod bitset;
pub mod board_metrics;
mod eval;
mod material;
pub mod memory;
mod moves;
mod position;

pub use attacks::{get_attackers_to, get_discovered_attacks};
pub use eval::flip_eval_perspective;
pub use material::{
    cap_eval_by_material, game_phase, has_insufficient_material, is_zugzwang, majors, minors,
    side_has_insufficient_material,
};
pub use moves::{
    collect_legal_moves, gives_check, has_check, has_legal_moves, is_capture, make_move, only_move,
};
pub use position::Position;
