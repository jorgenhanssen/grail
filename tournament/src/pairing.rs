use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    game::{GameArgs, GameRunner},
    openings::Opening,
    outcome::GameOutcome,
};
use rayon::prelude::*;

pub struct Pairing {
    openings: Vec<Opening>,
    engine_a: PathBuf,
    engine_b: PathBuf,
    move_time: u64,
}

impl Pairing {
    pub fn new(
        openings: Vec<Opening>,
        engine_a: PathBuf,
        engine_b: PathBuf,
        move_time: u64,
    ) -> Self {
        Self {
            openings,
            engine_a,
            engine_b,
            move_time,
        }
    }

    pub fn run(&self) -> Vec<GameOutcome> {
        let mut games = Vec::new();

        for opening in &self.openings {
            games.push((
                self.engine_a.clone(),
                self.engine_b.clone(),
                opening.clone(),
            ));

            games.push((
                self.engine_b.clone(),
                self.engine_a.clone(),
                opening.clone(),
            ));
        }

        let num_games = games.len();
        let progress = Arc::new(Mutex::new(0));

        games
            .par_iter()
            .filter_map(|(white, black, opening)| {
                let game_runner = GameRunner::new(white.clone(), black.clone());
                let game_args = GameArgs {
                    move_time: self.move_time,
                    opening: opening.clone(),
                };

                let outcome = game_runner.run(game_args).ok()?;

                let mut progress = progress.lock().unwrap();
                *progress += 1;

                log::info!("{} [{}/{}]", opening, *progress, num_games);
                log::info!("{}\n", outcome);

                Some(outcome)
            })
            .collect()
    }
}
