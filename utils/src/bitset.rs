#[derive(Clone, Copy)]
pub struct Bitset<const N: usize>([u64; N]);

impl<const N: usize> Bitset<N> {
    /// Set a bit at the given index.
    #[inline(always)]
    pub fn set(&mut self, idx: usize) {
        self.0[idx / 64] |= 1u64 << (idx % 64);
    }

    /// Check if a bit is set at the given index.
    #[inline(always)]
    pub fn get(&self, idx: usize) -> bool {
        (self.0[idx / 64] & (1u64 << (idx % 64))) != 0
    }

    /// Unset a bit at the given index.
    #[inline(always)]
    pub fn unset(&mut self, idx: usize) {
        self.0[idx / 64] &= !(1u64 << (idx % 64));
    }

    /// Toggle a bit at the given index.
    #[inline(always)]
    pub fn toggle(&mut self, idx: usize) {
        self.0[idx / 64] ^= 1u64 << (idx % 64);
    }

    /// Get the underlying array.
    #[inline(always)]
    pub fn as_array(&self) -> &[u64; N] {
        &self.0
    }
}

impl<const N: usize> Default for Bitset<N> {
    fn default() -> Self {
        Self([0; N])
    }
}
