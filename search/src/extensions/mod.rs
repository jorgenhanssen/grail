#[inline(always)]
pub fn check_extension(in_check: bool, is_pv_node: bool) -> u8 {
    // Extending all checks at any depth caused significant regression during testing.
    // Likely because exploding the search with insignificant checks and never-ending sequences.
    // Extending only in PV nodes should target the principal variation where tactics matter most,
    // reducing node overhead in non-PV parts of the tree.
    (in_check && is_pv_node) as u8
}
