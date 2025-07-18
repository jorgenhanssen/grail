use chess::Game;
use std::{collections::HashMap, error::Error, path::Path, str::FromStr};

use crate::{engine::EngineProcess, outcome::GameOutcome};

#[derive(Debug)]
pub struct GameArgs {
    pub move_time: u64,
    pub start_position_fen: String,
}

pub struct GameRunner {
    white: String,
    black: String,
}

impl GameRunner {
    pub fn new(white: impl AsRef<Path>, black: impl AsRef<Path>) -> Self {
        Self {
            white: white.as_ref().to_string_lossy().to_string(),
            black: black.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn run(&self, args: GameArgs) -> Result<GameOutcome, Box<dyn Error>> {
        let mut game =
            Game::from_str(&args.start_position_fen).map_err(|e| format!("Invalid FEN: {}", e))?;

        let mut white_engine = EngineProcess::new(&self.white)?;
        let mut black_engine = EngineProcess::new(&self.black)?;

        let mut moves_played = Vec::new();
        let mut positions = Vec::new();
        let mut position_counts = HashMap::new();

        let initial_board = game.current_position();
        positions.push(initial_board);
        *position_counts.entry(initial_board).or_insert(0) += 1;

        while game.result().is_none() {
            let board = game.current_position();
            let engine = match board.side_to_move() {
                chess::Color::White => &mut white_engine,
                chess::Color::Black => &mut black_engine,
            };
            let mv = engine.best_move(&board.to_string(), args.move_time)?;

            game.make_move(mv);
            moves_played.push(mv);

            let new_board = game.current_position();
            positions.push(new_board);
            *position_counts.entry(new_board).or_insert(0) += 1;

            if position_counts[&new_board] >= 3 {
                game.declare_draw();
            }
        }

        let result = game.result().ok_or("Game ended without a result")?;

        Ok(GameOutcome {
            starting_position: args.start_position_fen,
            white_name: self.white.clone(),
            black_name: self.black.clone(),
            result,
            moves: moves_played,
            positions,
        })
    }
}
