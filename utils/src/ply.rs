use std::ops::{Add, AddAssign, Div, Sub, SubAssign};

/// A fractional ply for fine-grained reduction calculations.
/// Internal units: 1024 = 1 ply.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct FracPly(pub i32);

impl FracPly {
    /// Number of fractional units per ply.
    pub const ONE: i32 = 1024;

    /// Convert signed whole plies to fixed-point units.
    pub const fn from_ply(plies: i32) -> Self {
        Self(plies * Self::ONE)
    }

    /// Number of whole plies.
    pub const fn whole(self) -> i32 {
        self.0 / Self::ONE
    }
}

impl Add for FracPly {
    type Output = FracPly;

    fn add(self, rhs: FracPly) -> FracPly {
        FracPly(self.0 + rhs.0)
    }
}

impl AddAssign for FracPly {
    fn add_assign(&mut self, rhs: FracPly) {
        self.0 += rhs.0;
    }
}

impl Sub for FracPly {
    type Output = FracPly;

    fn sub(self, rhs: FracPly) -> FracPly {
        FracPly(self.0 - rhs.0)
    }
}

impl SubAssign for FracPly {
    fn sub_assign(&mut self, rhs: FracPly) {
        self.0 -= rhs.0;
    }
}

impl Div<i32> for FracPly {
    type Output = FracPly;

    fn div(self, rhs: i32) -> FracPly {
        FracPly(self.0 / rhs)
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
        assert_eq!(FracPly(-one).whole(), -1);
        assert_eq!(FracPly(-one - 1).whole(), -1);
    }

    #[test]
    fn sub_goes_negative() {
        let a = FracPly(100);
        let b = FracPly(200);
        assert_eq!((a - b).0, -100);
        assert_eq!((a - b).whole(), 0);
    }

    #[test]
    fn sub_negative_increases() {
        let mut a = FracPly::from_ply(1);
        a -= FracPly::from_ply(-1);
        assert_eq!(a.0, FracPly::ONE * 2);
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
