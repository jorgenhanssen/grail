/// Internal Iterative Reductions: reduce depth when no TT move is found.
///
/// When no hash move is found, reduce the search depth instead of doing a
/// full-depth search with poor move ordering.
///
/// <https://www.chessprogramming.org/Internal_Iterative_Reductions>
pub fn iir(
    max_depth: u8,
    remaining_depth: u8,
    has_tt_move: bool,
    min_depth: u8,
    reduction: u8,
) -> (u8, u8) {
    if !has_tt_move && remaining_depth >= min_depth {
        (
            max_depth.saturating_sub(reduction),
            remaining_depth.saturating_sub(reduction),
        )
    } else {
        (max_depth, remaining_depth)
    }
}
