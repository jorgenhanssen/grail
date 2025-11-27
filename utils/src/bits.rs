/// Set a bit at the given index.
#[inline(always)]
pub fn set_bit(bits: &mut [u64], idx: usize) {
    bits[idx / 64] |= 1u64 << (idx % 64);
}

/// Check if a bit is set at the given index.
#[inline(always)]
pub fn get_bit(bits: &[u64], idx: usize) -> bool {
    (bits[idx / 64] & (1u64 << (idx % 64))) != 0
}

/// Clear a bit at the given index.
#[inline(always)]
pub fn clear_bit(bits: &mut [u64], idx: usize) {
    bits[idx / 64] &= !(1u64 << (idx % 64));
}

/// Toggle a bit at the given index.
#[inline(always)]
pub fn toggle_bit(bits: &mut [u64], idx: usize) {
    bits[idx / 64] ^= 1u64 << (idx % 64);
}
