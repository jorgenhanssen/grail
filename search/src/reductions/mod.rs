mod iir;
mod lmr;

pub use iir::iir;
pub use lmr::lmr;

pub fn cap_reduction(reduction: u8, remaining_depth: u8, ratio: f32) -> u8 {
    let max_reduction = (remaining_depth as f32 * ratio) as u8;
    reduction.min(max_reduction)
}

pub fn can_reduce(tactical: bool, is_pv_move: bool) -> bool {
    !tactical && !is_pv_move
}
