use std::{collections::HashMap, fmt};

use chess::GameResult;

use crate::outcome::GameOutcome;

#[derive(Default)]
struct EngineSummary {
    wins_as_white: u32,
    wins_as_black: u32,
    draws: u32,
    games_played: u32,
}

impl EngineSummary {
    fn score(&self) -> f64 {
        (self.wins_as_white + self.wins_as_black) as f64 + (self.draws as f64 * 0.5)
    }

    fn total_wins(&self) -> u32 {
        self.wins_as_white + self.wins_as_black
    }
}

pub struct Summary {
    engines: HashMap<String, EngineSummary>,
    total_games: u32,
}

impl Summary {
    pub fn new(outcomes: &Vec<GameOutcome>) -> Self {
        let mut engines = HashMap::<String, EngineSummary>::new();

        for outcome in outcomes {
            let white_entry = engines.entry(outcome.white_name.clone()).or_default();
            let black_entry = engines.entry(outcome.black_name.clone()).or_default();

            // Increment games played for both engines
            white_entry.games_played += 1;
            black_entry.games_played += 1;

            // Update statistics based on game result
            match outcome.result {
                GameResult::WhiteCheckmates | GameResult::BlackResigns => {
                    white_entry.wins_as_white += 1;
                }
                GameResult::BlackCheckmates | GameResult::WhiteResigns => {
                    black_entry.wins_as_black += 1;
                }
                GameResult::DrawDeclared | GameResult::DrawAccepted | GameResult::Stalemate => {
                    white_entry.draws += 1;
                    black_entry.draws += 1;
                }
            }
        }

        Summary {
            engines,
            total_games: outcomes.len() as u32,
        }
    }

    pub fn get_engine_summary(&self, engine_name: &str) -> Option<&EngineSummary> {
        self.engines.get(engine_name)
    }

    pub fn get_sorted_engines(&self) -> Vec<(&String, &EngineSummary)> {
        let mut engines: Vec<_> = self.engines.iter().collect();
        engines.sort_by(|a, b| {
            b.1.score()
                .partial_cmp(&a.1.score())
                .unwrap_or(std::cmp::Ordering::Equal)
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
            let score = summary.score();
            let win_rate = if summary.games_played > 0 {
                (summary.total_wins() as f64 / summary.games_played as f64) * 100.0
            } else {
                0.0
            };

            writeln!(f, "{}. Engine: {}", rank + 1, engine_name)?;
            writeln!(f, "   Games Played: {}", summary.games_played)?;
            writeln!(f, "   Wins as White: {}", summary.wins_as_white)?;
            writeln!(f, "   Wins as Black: {}", summary.wins_as_black)?;
            writeln!(f, "   Draws: {}", summary.draws)?;
            writeln!(
                f,
                "   Score: {:.1}/{} ({:.1}%)",
                score,
                summary.games_played,
                (score / summary.games_played as f64) * 100.0
            )?;
            writeln!(f, "   Win Rate: {:.1}%", win_rate)?;
            writeln!(f)?;
        }

        Ok(())
    }
}
