/// A packed bit array of `WORDS` u64s.
#[derive(Clone, Copy)]
pub struct Bitset<const WORDS: usize>([u64; WORDS]);

/// Creates a new Bitset for a specified number of bits.
#[macro_export]
macro_rules! bitset {
    ($bits:expr) => {
        $crate::bitset::Bitset<{ ($bits as usize).div_ceil(64) }>
    };
}

impl<const WORDS: usize> Bitset<WORDS> {
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

    /// Get the index of the u64 that represents the given bit offset.
    pub const fn u64_index(&self, bit_offset: usize) -> usize {
        bit_offset / 64
    }

    /// Set a u64 at the given u64 by index.
    pub fn set_u64(&mut self, index: usize, value: u64) {
        self.0[index] = value;
    }

    /// Get the underlying u64 array.
    pub fn as_array(&self) -> &[u64; WORDS] {
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
                f(word_idx * 64 + bit_idx);
            }
        }
    }
}

impl<const WORDS: usize> Default for Bitset<WORDS> {
    fn default() -> Self {
        Self([0; WORDS])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut bits: bitset!(128) = Bitset::default();
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
        let mut bits: bitset!(64) = Bitset::default();
        bits.set(10);
        assert!(bits.get(10));

        bits.unset(10);
        assert!(!bits.get(10));
    }

    #[test]
    fn test_toggle() {
        let mut bits: bitset!(64) = Bitset::default();
        assert!(!bits.get(5));

        bits.toggle(5);
        assert!(bits.get(5));

        bits.toggle(5);
        assert!(!bits.get(5));
    }

    #[test]
    fn test_cross_word_boundary() {
        let mut bits: bitset!(128) = Bitset::default();
        bits.set(63);
        bits.set(64);

        assert!(bits.get(63));
        assert!(bits.get(64));
        assert!(!bits.get(62));
        assert!(!bits.get(65));
    }

    #[test]
    fn test_for_each_diff() {
        let mut a: bitset!(128) = Bitset::default();
        let mut b: bitset!(128) = Bitset::default();

        a.set(0);
        a.set(10);
        a.set(64);

        b.set(10);
        b.set(64);
        b.set(100);

        let mut diffs = Vec::new();
        a.for_each_diff(&b, |idx| diffs.push(idx));

        assert_eq!(diffs.len(), 2);
        assert!(diffs.contains(&0));
        assert!(diffs.contains(&100));
    }

    #[test]
    fn test_default_is_all_zeros() {
        let bits: bitset!(256) = Bitset::default();
        for i in 0..256 {
            assert!(!bits.get(i));
        }
    }

    #[test]
    fn test_u64_index() {
        let bits: bitset!(256) = Bitset::default();
        assert_eq!(bits.u64_index(0), 0);
        assert_eq!(bits.u64_index(63), 0);
        assert_eq!(bits.u64_index(64), 1);
        assert_eq!(bits.u64_index(127), 1);
        assert_eq!(bits.u64_index(128), 2);
    }

    #[test]
    fn test_set_u64() {
        let mut bits: bitset!(256) = Bitset::default();
        bits.set_u64(1, u64::MAX);

        for i in 0..64 {
            assert!(!bits.get(i));
        }
        for i in 64..128 {
            assert!(bits.get(i));
        }
        for i in 128..256 {
            assert!(!bits.get(i));
        }
    }

    #[test]
    fn test_set_u64_specific_pattern() {
        let mut bits: bitset!(128) = Bitset::default();
        bits.set_u64(0, 0xAAAAAAAAAAAAAAAA);

        for i in 0..64 {
            assert_eq!(bits.get(i), i % 2 == 1);
        }
    }
}
