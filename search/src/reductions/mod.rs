mod iir;
mod lmr;

pub use iir::iir;
pub use lmr::lmr;

pub fn cap_reduction(reduction: u8, remaining_depth: u8) -> u8 {
    reduction.min(remaining_depth - 2) // Search at least 2 ply
}
