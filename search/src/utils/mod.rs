mod bounds;
mod score;
pub mod see;

pub use bounds::Bounds;
pub use score::{convert_centipawn_score, convert_mate_score};

pub fn near_root(ply: u8, depth: u8) -> bool {
    let total_depth = depth + ply;
    ply <= total_depth >> 3 // 12.5% of the total depth
}
