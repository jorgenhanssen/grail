/// A packed bit array that holds `BITS` bits.
///
/// The bits are stored in an array of `u64` values, automatically sized
/// to fit the requested number of bits.
#[derive(Clone, Copy)]
pub struct Bitset<const BITS: usize>([u64; BITS.div_ceil(64)])
where
    [(); BITS.div_ceil(64)]:;

impl<const BITS: usize> Bitset<BITS>
where
    [(); BITS.div_ceil(64)]:,
{
    /// Set a bit at the given index.
    pub fn set(&mut self, idx: usize) {
        self.0[idx / 64] |= 1u64 << (idx % 64);
    }

    /// Check if a bit is set at the given index.
    pub fn get(&self, idx: usize) -> bool {
        (self.0[idx / 64] & (1u64 << (idx % 64))) != 0
    }

    /// Unset a bit at the given index.
    pub fn unset(&mut self, idx: usize) {
        self.0[idx / 64] &= !(1u64 << (idx % 64));
    }

    /// Toggle a bit at the given index.
    pub fn toggle(&mut self, idx: usize) {
        self.0[idx / 64] ^= 1u64 << (idx % 64);
    }

    /// Get the underlying `u64` array.
    pub fn as_array(&self) -> &[u64; BITS.div_ceil(64)] {
        &self.0
    }

    /// Call a function for each index where bits differ between self and other.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut bits: Bitset<128> = Bitset::default();
        assert!(!bits.get(0));
        assert!(!bits.get(127));

        bits.set(0);
        bits.set(127);

        assert!(bits.get(0));
        assert!(bits.get(127));
        assert!(!bits.get(1));
        assert!(!bits.get(64));
    }

    #[test]
    fn test_unset() {
        let mut bits: Bitset<64> = Bitset::default();
        bits.set(10);
        assert!(bits.get(10));

        bits.unset(10);
        assert!(!bits.get(10));
    }

    #[test]
    fn test_toggle() {
        let mut bits: Bitset<64> = Bitset::default();
        assert!(!bits.get(5));

        bits.toggle(5);
        assert!(bits.get(5));

        bits.toggle(5);
        assert!(!bits.get(5));
    }

    #[test]
    fn test_cross_word_boundary() {
        let mut bits: Bitset<128> = Bitset::default();

        // Set bits at word boundaries
        bits.set(63); // Last bit of first word
        bits.set(64); // First bit of second word

        assert!(bits.get(63));
        assert!(bits.get(64));
        assert!(!bits.get(62));
        assert!(!bits.get(65));
    }

    #[test]
    fn test_for_each_diff() {
        let mut a: Bitset<128> = Bitset::default();
        let mut b: Bitset<128> = Bitset::default();

        a.set(0);
        a.set(10);
        a.set(64);

        b.set(10);
        b.set(64);
        b.set(100);

        let mut diffs = Vec::new();
        a.for_each_diff(&b, |idx| diffs.push(idx));

        // Bit 0: in a, not in b
        // Bit 100: in b, not in a
        // Bits 10 and 64: same in both (no diff)
        assert_eq!(diffs.len(), 2);
        assert!(diffs.contains(&0));
        assert!(diffs.contains(&100));
    }

    #[test]
    fn test_default_is_all_zeros() {
        let bits: Bitset<256> = Bitset::default();
        for i in 0..256 {
            assert!(!bits.get(i));
        }
    }
}
