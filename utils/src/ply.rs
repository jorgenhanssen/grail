use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A fractional ply for fine-grained reduction calculations.
/// Internal units: 1024 = 1 ply.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct FracPly(pub u16);

impl FracPly {
    /// Number of fractional units per ply.
    pub const ONE: u16 = 1024;

    /// Number of whole plies.
    #[inline]
    pub const fn whole(self) -> u8 {
        (self.0 / Self::ONE) as u8
    }
}

impl Add for FracPly {
    type Output = FracPly;

    #[inline]
    fn add(self, rhs: FracPly) -> FracPly {
        FracPly(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for FracPly {
    #[inline]
    fn add_assign(&mut self, rhs: FracPly) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for FracPly {
    type Output = FracPly;

    #[inline]
    fn sub(self, rhs: FracPly) -> FracPly {
        FracPly(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for FracPly {
    #[inline]
    fn sub_assign(&mut self, rhs: FracPly) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_truncates() {
        let one = FracPly::ONE;
        let half = one / 2;

        assert_eq!(FracPly(0).whole(), 0);
        assert_eq!(FracPly(half).whole(), 0);
        assert_eq!(FracPly(one - 1).whole(), 0);
        assert_eq!(FracPly(one).whole(), 1);
        assert_eq!(FracPly(one + half).whole(), 1);
        assert_eq!(FracPly(one * 2).whole(), 2);
    }

    #[test]
    fn add_saturates() {
        let a = FracPly(u16::MAX - 100);
        let b = FracPly(200);
        assert_eq!((a + b).0, u16::MAX);
    }

    #[test]
    fn sub_saturates() {
        let a = FracPly(100);
        let b = FracPly(200);
        assert_eq!((a - b).0, 0);
    }

    #[test]
    fn add_assign() {
        let one = FracPly::ONE;
        let half = one / 2;

        let mut a = FracPly(one);
        a += FracPly(half);
        assert_eq!(a.0, one + half);
    }

    #[test]
    fn sub_assign() {
        let one = FracPly::ONE;
        let half = one / 2;

        let mut a = FracPly(one);
        a -= FracPly(half);
        assert_eq!(a.0, half);
    }

    #[test]
    fn ordering() {
        assert!(FracPly(100) < FracPly(200));
        assert!(FracPly(200) > FracPly(100));
        assert!(FracPly(100) == FracPly(100));
    }
}
