/// Internal Iterative Reductions: reduce depth when no TT move is found.
///
/// When no hash move is found, reduce the search depth instead of doing a
/// full-depth search with poor move ordering. Skipped at root to preserve
/// iteration depth.
///
/// <https://www.chessprogramming.org/Internal_Iterative_Reductions>
pub fn iir(ply: u8, depth: u8, has_tt_move: bool, min_depth: u8, reduction: u8) -> u8 {
    if ply > 0 && !has_tt_move && depth >= min_depth {
        depth.saturating_sub(reduction)
    } else {
        depth
    }
}
