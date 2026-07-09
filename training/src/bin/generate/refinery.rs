use crate::game::Position;
use crate::samples::{GameOutcome, Sample};
use cozy_chess::Board;
use pyrrhic_rs::{TableBases, WdlProbeResult};
use search::{CozyAdapter, tablebase};

/// Position/sample cleanup:
/// - re-label outcomes based on tb
/// - TODO: eval / outcome disagreements
pub struct Refinery {
    tablebases: Option<TableBases<CozyAdapter>>,
    stats: RefinementStats,
}

#[derive(Default, Clone, Copy)]
pub struct RefinementStats {
    /// Number of games entering tb range and got labels from it.
    games_labeled_by_tb: usize,
    /// Games where the tb outcome changed between probes
    /// (basically a thrown win/draw somewhere in tb range).
    games_with_tb_deviations: usize,
}

impl RefinementStats {
    pub fn merge(&mut self, other: &RefinementStats) {
        self.games_labeled_by_tb += other.games_labeled_by_tb;
        self.games_with_tb_deviations += other.games_with_tb_deviations;
    }

    pub fn log_summary(&self) {
        if self.games_labeled_by_tb > 0 {
            log::info!(
                "TB outcomes: {} games labeled from tablebases, {} with deviations ({:.2}%)",
                self.games_labeled_by_tb,
                self.games_with_tb_deviations,
                self.games_with_tb_deviations as f64 * 100.0 / self.games_labeled_by_tb as f64,
            );
        }
    }
}

impl Refinery {
    pub fn new(tablebases: Option<TableBases<CozyAdapter>>) -> Self {
        Self {
            tablebases,
            stats: RefinementStats::default(),
        }
    }

    pub fn stats(&self) -> RefinementStats {
        self.stats
    }

    pub fn refine(
        &mut self,
        game_id: usize,
        positions: Vec<Position>,
        proven_outcome: Option<GameOutcome>,
    ) -> Vec<Sample> {
        let tb_outcomes: Vec<Option<GameOutcome>> =
            positions.iter().map(|p| self.tb_probe(p)).collect();

        let (outcomes, stats) = resolve_outcomes(&tb_outcomes, proven_outcome);

        self.stats.merge(&stats);

        positions
            .into_iter()
            .zip(outcomes)
            .map(|(position, outcome)| Sample {
                fen: position.fen,
                score: position.score,
                game_id,
                best_move: position.best_move,
                outcome,
            })
            .collect()
    }

    fn tb_probe(&self, position: &Position) -> Option<GameOutcome> {
        let board: Board = position.fen.parse().ok()?;
        if board.halfmove_clock() != 0 {
            return None;
        }

        let tb = self.tablebases.as_ref()?;
        let wdl = tablebase::probe_wdl(tb, &board)?;

        Some(match wdl {
            WdlProbeResult::Win => GameOutcome::win(board.side_to_move()),
            WdlProbeResult::Loss => GameOutcome::win(!board.side_to_move()),
            _ => GameOutcome::Draw,
        })
    }
}

/// Each position gets the tb outcome of the next probe.
///
/// NB! If the outcome is speculative (50-move rule, repetition), then we
/// use the last tb outcome for the remainder of the game. That way a won
/// game shuffling/teleporting into a draw still gets labeled as a win.
fn resolve_outcomes(
    tb_outcomes: &[Option<GameOutcome>],
    proven_outcome: Option<GameOutcome>,
) -> (Vec<GameOutcome>, RefinementStats) {
    let last_tb_outcome = tb_outcomes.iter().flatten().last().copied();

    let mut outcome = proven_outcome
        .or(last_tb_outcome)
        .unwrap_or(GameOutcome::Draw);

    let mut outcomes = vec![outcome; tb_outcomes.len()];
    for (resolved, tb_outcome) in outcomes.iter_mut().zip(tb_outcomes).rev() {
        if let Some(tb_outcome) = tb_outcome {
            outcome = *tb_outcome;
        }
        *resolved = outcome;
    }

    let known: Vec<_> = tb_outcomes.iter().flatten().collect();
    let deviated = known.windows(2).any(|pair| pair[0] != pair[1]);

    let stats = RefinementStats {
        games_labeled_by_tb: last_tb_outcome.is_some() as usize,
        games_with_tb_deviations: deviated as usize,
    };

    (outcomes, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use GameOutcome::{Black, Draw, White};

    #[test]
    fn game_outside_the_tables_is_a_rule_draw() {
        let (outcomes, stats) = resolve_outcomes(&[None; 3], None);

        assert_eq!(outcomes, [Draw, Draw, Draw]);
        assert_eq!(stats.games_labeled_by_tb, 0);
        assert_eq!(stats.games_with_tb_deviations, 0);
    }

    #[test]
    fn mate_on_the_board_labels_everything() {
        let (outcomes, _) = resolve_outcomes(&[None; 3], Some(White));

        assert_eq!(outcomes, [White, White, White]);
    }

    #[test]
    fn tb_outcome_covers_the_positions_leading_up_to_it() {
        let (outcomes, stats) = resolve_outcomes(&[None, None, Some(White), None], None);

        assert_eq!(outcomes, [White, White, White, White]);
        assert_eq!(stats.games_labeled_by_tb, 1);
        assert_eq!(stats.games_with_tb_deviations, 0);
    }

    #[test]
    fn won_game_shuffled_into_repetition_is_still_a_win() {
        let (outcomes, _) = resolve_outcomes(&[Some(Black), None, None], None);

        assert_eq!(outcomes, [Black, Black, Black]);
    }

    #[test]
    fn thrown_win_splits_the_game_into_spans() {
        let (outcomes, stats) =
            resolve_outcomes(&[None, Some(White), None, Some(Draw), None], None);

        assert_eq!(outcomes, [White, White, Draw, Draw, Draw]);
        assert_eq!(stats.games_with_tb_deviations, 1);
    }

    #[test]
    fn blunder_after_the_last_probe_trumps_the_tables() {
        let (outcomes, _) = resolve_outcomes(&[Some(Draw), None, None], Some(Black));

        assert_eq!(outcomes, [Draw, Black, Black]);
    }
}
