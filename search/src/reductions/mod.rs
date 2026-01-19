mod iir;
mod lmr;

pub use iir::iir;
pub use lmr::LmrTable;

pub fn cap_reduction(reduction: u8, depth: u8) -> u8 {
    reduction.min(depth.saturating_sub(2)) // Search at least 2 ply
}
