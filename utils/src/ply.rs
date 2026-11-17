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

    /// Nearest whole ply
    pub const fn round(self) -> i32 {
        let value = self.0 as i64;
        let half = Self::ONE as i64 / 2;
        ((value + value.signum() * half) / Self::ONE as i64) as i32
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
