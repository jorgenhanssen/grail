use utils::select_softmax;

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
    pub fn new(lines: Vec<PvLine>) -> Self {
        Self { lines }
    }

    /// Returns the primary (best) PV line.
    pub fn primary(&self) -> Option<&PvLine> {
        self.lines.first()
    }

    /// Returns all PV lines.
    pub fn lines(&self) -> &[PvLine] {
        &self.lines
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

        // Scale scores down so centipawn differences give reasonable probabilities
        // e.g., 50cp difference -> 0.5 units -> exp(-0.5) ≈ 0.6
        let scores: Vec<f32> = self
            .lines
            .iter()
            .map(|pv| pv.score as f32 / SOFTMAX_SCALE)
            .collect();

        let idx = select_softmax(&scores, &mut rand::thread_rng());
        Some(&self.lines[idx])
    }
}
