use rand::Rng;

use crate::pv::PvLine;

/// Scale factor for softmax over centipawn scores.
/// Dividing by this makes typical move differences (~20-50cp) produce
/// reasonable probabilities rather than near-deterministic selection.
const SOFTMAX_SCALE: f32 = 100.0;

/// Result of a search containing all PV lines found.
///
/// When MultiPV > 1, contains multiple lines ranked by quality.
/// Provides convenient accessors for the primary line and selection methods.
#[derive(Clone, Debug, Default)]
pub struct SearchResult {
    lines: Vec<PvLine>,
}

impl SearchResult {
    /// Creates a new search result from PV lines.
    pub fn new(lines: Vec<PvLine>) -> Self {
        Self { lines }
    }

    /// Returns the primary (best) PV line, if any.
    pub fn primary(&self) -> Option<&PvLine> {
        self.lines.first()
    }

    /// Returns all PV lines.
    pub fn lines(&self) -> &[PvLine] {
        &self.lines
    }

    /// Returns true if no PV lines were found.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Select a PV line using softmax over scores.
    ///
    /// Higher-scoring lines are more likely to be selected.
    /// Scores are scaled down so typical differences give reasonable probabilities.
    /// Returns None if no lines available.
    pub fn select_softmax(&self) -> Option<&PvLine> {
        if self.lines.is_empty() {
            return None;
        }

        if self.lines.len() == 1 {
            return self.primary();
        }

        let mut rng = rand::thread_rng();

        // Scale scores down so centipawn differences give reasonable probabilities
        // e.g., 50cp difference -> 0.5 units -> exp(-0.5) ≈ 0.6
        let scores: Vec<f32> = self
            .lines
            .iter()
            .map(|pv| pv.score as f32 / SOFTMAX_SCALE)
            .collect();

        // Find max for numerical stability
        let max_score = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Compute softmax weights
        let weights: Vec<f32> = scores.iter().map(|&s| (s - max_score).exp()).collect();

        let total: f32 = weights.iter().sum();

        // Sample from the distribution
        let mut r = rng.gen::<f32>() * total;
        for (i, &w) in weights.iter().enumerate() {
            r -= w;
            if r <= 0.0 {
                return Some(&self.lines[i]);
            }
        }

        // Fallback (shouldn't happen with valid weights)
        self.lines.last()
    }
}
