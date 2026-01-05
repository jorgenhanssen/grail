mod score;
pub mod see;

pub use score::{convert_centipawn_score, convert_mate_score};

pub fn near_root(depth: u8, remaining_depth: u8) -> bool {
    let total_depth = remaining_depth + depth;
    depth <= total_depth >> 3 // 12.5% of the total depth
}
