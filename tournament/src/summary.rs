use std::{collections::HashMap, fmt};

use chess::{Color, GameResult};

use crate::outcome::GameOutcome;

#[derive(Default)]
struct EngineSummary {
    wins_as_white: u32,
    wins_as_black: u32,
    draws: u32,
    num_games: u32,
}

impl EngineSummary {
    fn record_game(&mut self, result: GameResult, playing_as: Color) {
        self.num_games += 1;

        if is_draw(result) {
            self.draws += 1;
        } else if is_win_for(result, playing_as) {
            match playing_as {
                Color::White => self.wins_as_white += 1,
                Color::Black => self.wins_as_black += 1,
            }
        }
    }

    #[inline]
    fn score(&self) -> f64 {
        self.total_wins() as f64 + self.draws as f64 * 0.5
    }

    #[inline]
    fn total_wins(&self) -> u32 {
        self.wins_as_white + self.wins_as_black
    }

    #[inline]
    fn win_rate(&self) -> f64 {
        if self.num_games == 0 {
            0.0
        } else {
            (self.total_wins() as f64 / self.num_games as f64) * 100.0
        }
    }
}

pub struct Summary {
    engines: HashMap<String, EngineSummary>,
    total_games: u32,
}

impl Summary {
    pub fn new(outcomes: &[GameOutcome]) -> Self {
        let mut engines = HashMap::new();

        for outcome in outcomes {
            // Record game for white engine
            engines
                .entry(outcome.white_name.clone())
                .or_insert_with(EngineSummary::default)
                .record_game(outcome.result, Color::White);

            // Record game for black engine
            engines
                .entry(outcome.black_name.clone())
                .or_insert_with(EngineSummary::default)
                .record_game(outcome.result, Color::Black);
        }

        Summary {
            engines,
            total_games: outcomes.len() as u32,
        }
    }

    #[inline]
    fn get_sorted_engines(&self) -> Vec<(&String, &EngineSummary)> {
        let mut engines: Vec<_> = self.engines.iter().collect();
        engines.sort_by(|a, b| {
            b.1.score()
                .partial_cmp(&a.1.score())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        engines
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tournament Summary")?;
        writeln!(f, "==================")?;
        writeln!(f, "Total Games: {}", self.total_games)?;
        writeln!(f)?;

        let sorted_engines = self.get_sorted_engines();

        for (rank, (engine_name, summary)) in sorted_engines.iter().enumerate() {
            writeln!(f, "{}. Engine: {}", rank + 1, engine_name)?;
            writeln!(f, "   Wins as White: {}", summary.wins_as_white)?;
            writeln!(f, "   Wins as Black: {}", summary.wins_as_black)?;
            writeln!(f, "   Draws: {}", summary.draws)?;
            writeln!(f, "   Score: {:.1}/{}", summary.score(), summary.num_games)?;
            writeln!(f, "   Win Rate: {:.1}%", summary.win_rate())?;
            writeln!(f)?;
        }

        Ok(())
    }
}

#[inline]
fn is_win_for(result: GameResult, color: Color) -> bool {
    match (color, result) {
        (Color::White, GameResult::WhiteCheckmates | GameResult::BlackResigns) => true,
        (Color::Black, GameResult::BlackCheckmates | GameResult::WhiteResigns) => true,
        _ => false,
    }
}

#[inline]
fn is_draw(result: GameResult) -> bool {
    matches!(
        result,
        GameResult::DrawAccepted | GameResult::DrawDeclared | GameResult::Stalemate
    )
}
