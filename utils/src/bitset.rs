/// A packed bit array that holds `BITS` bits.
///
/// The bits are stored in an array of `u64` values, automatically sized
/// to fit the requested number of bits.
///
/// # Example
/// ```
/// use utils::bitset::Bitset;
///
/// let mut bits: Bitset<128> = Bitset::default();
/// bits.set(0);
/// bits.set(127);
/// assert!(bits.get(0));
/// assert!(bits.get(127));
/// assert!(!bits.get(1));
/// ```
#[derive(Clone, Copy)]
pub struct Bitset<const BITS: usize>([u64; BITS.div_ceil(64)])
where
    [(); BITS.div_ceil(64)]:;

impl<const BITS: usize> Bitset<BITS>
where
    [(); BITS.div_ceil(64)]:,
{
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

    /// Get the underlying `u64` array.
    #[inline(always)]
    pub fn as_array(&self) -> &[u64; BITS.div_ceil(64)] {
        &self.0
    }

    /// Call a function for each index where bits differ between self and other.
    #[inline(always)]
    pub fn for_each_diff<F>(&self, other: &Self, mut f: F)
    where
        F: FnMut(usize),
    {
        for (word_idx, (&self_word, &other_word)) in self.0.iter().zip(other.0.iter()).enumerate() {
            let mut changes = self_word ^ other_word;
            while changes != 0 {
                let bit_idx = changes.trailing_zeros() as usize;
                changes &= changes - 1;
                let idx = word_idx * 64 + bit_idx;
                if idx < BITS {
                    f(idx);
                }
            }
        }
    }
}

impl<const BITS: usize> Default for Bitset<BITS>
where
    [(); BITS.div_ceil(64)]:,
{
    fn default() -> Self {
        Self([0; BITS.div_ceil(64)])
    }
}
