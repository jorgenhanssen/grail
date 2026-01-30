use utils::select_softmax;

use crate::pv::PvLine;

/// Softmax temperature: higher = more exploration, lower = best move.
/// Examples at T=100 with 3 moves: [20,21,21]cp => [33,33,33]%, [484,427,422]cp => [48,27,26]%.
/// See: https://www.baeldung.com/cs/softmax-temperature
const SOFTMAX_TEMPERATURE: f32 = 100.0;

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
    /// Returns None if no lines are available.
    pub fn select_softmax(&self) -> Option<&PvLine> {
        if self.lines.is_empty() {
            return None;
        }

        if self.lines.len() == 1 {
            return self.primary();
        }

        let scores: Vec<f32> = self
            .lines
            .iter()
            .map(|pv| pv.score as f32 / SOFTMAX_TEMPERATURE)
            .collect();

        let idx = select_softmax(&scores, &mut rand::thread_rng());
        Some(&self.lines[idx])
    }
}
