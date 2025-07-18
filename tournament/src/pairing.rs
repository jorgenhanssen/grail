use std::path::PathBuf;

use crate::{
    game::{GameArgs, GameRunner},
    outcome::GameOutcome,
};
use rayon::prelude::*;

pub struct Pairing {
    positions: Vec<String>,
    engine_a: PathBuf,
    engine_b: PathBuf,
    move_time: u64,
}

impl Pairing {
    pub fn new(
        positions: Vec<String>,
        engine_a: PathBuf,
        engine_b: PathBuf,
        move_time: u64,
    ) -> Self {
        Self {
            positions,
            engine_a,
            engine_b,
            move_time,
        }
    }

    pub fn run(&self) -> Vec<GameOutcome> {
        let mut games = Vec::new();

        for position in &self.positions {
            games.push((
                self.engine_a.clone(),
                self.engine_b.clone(),
                position.clone(),
            ));

            games.push((
                self.engine_b.clone(),
                self.engine_a.clone(),
                position.clone(),
            ));
        }

        games
            .par_iter()
            .filter_map(|(white, black, position)| {
                let game_runner = GameRunner::new(white.clone(), black.clone());
                let game_args = GameArgs {
                    move_time: self.move_time,
                    start_position_fen: position.clone(),
                };

                let outcome = game_runner.run(game_args).ok()?;

                log::info!("{}", outcome);

                Some(outcome)
            })
            .collect()
    }
}
