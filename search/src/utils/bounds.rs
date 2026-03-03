use crate::scores::SCORE_INF;

/// Alpha-beta search bounds.
///
/// In negamax, `alpha` is the best score the maximizing player can guarantee,
/// and `beta` is the best score the opponent can guarantee. The search tries
/// to find a score in the window (alpha, beta).
///
/// <https://www.chessprogramming.org/Alpha-Beta>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub alpha: i16,
    pub beta: i16,
}

impl Bounds {
    /// Full window: searches all possible scores.
    pub const FULL: Self = Self {
        alpha: -SCORE_INF,
        beta: SCORE_INF,
    };

    /// Creates a new bounds with the given alpha and beta.
    pub fn new(alpha: i16, beta: i16) -> Self {
        Self { alpha, beta }
    }

    /// Creates a null (zero-width) window around alpha.
    /// Used in PVS for non-PV moves to quickly prove they're worse than alpha.
    pub fn null(alpha: i16) -> Self {
        Self {
            alpha,
            beta: alpha + 1,
        }
    }

    /// Inverts the bounds for child search (negamax).
    /// The child's alpha is -beta and beta is -alpha.
    pub fn invert(&self) -> Self {
        Self {
            alpha: -self.beta,
            beta: -self.alpha,
        }
    }

    /// Returns true if score causes a beta cutoff.
    pub fn is_cutoff(&self, score: i16) -> bool {
        score >= self.beta
    }

    /// Raises alpha if score is higher (found a better move).
    pub fn raise_alpha(&mut self, score: i16) {
        self.alpha = self.alpha.max(score);
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::FULL
    }
}
