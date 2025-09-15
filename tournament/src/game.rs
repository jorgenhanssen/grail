use chess::Game;
use std::{collections::HashMap, error::Error, path::Path, str::FromStr};

use crate::{
    args::TimeControlType, engine::EngineProcess, openings::Opening, outcome::GameOutcome,
};

#[derive(Debug)]
pub struct GameArgs {
    pub time_control: TimeControlType,
    pub opening: Opening,
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
        log::debug!(
            "Starting game: {} vs {} with opening {}",
            self.white,
            self.black,
            args.opening.name
        );
        log::debug!("Time control: {:?}", args.time_control);

        let mut game =
            Game::from_str(args.opening.fen).map_err(|e| format!("Invalid FEN: {}", e))?;

        log::debug!("Creating engine processes...");
        let mut white_engine = EngineProcess::new(&self.white)
            .map_err(|e| format!("Failed to start white engine {}: {}", self.white, e))?;
        let mut black_engine = EngineProcess::new(&self.black)
            .map_err(|e| format!("Failed to start black engine {}: {}", self.black, e))?;
        log::debug!("Engine processes created successfully");

        // Initialize time controls
        let (mut white_time, mut black_time, increment) = match &args.time_control {
            TimeControlType::TimeControl { inc, time } => (*time, *time, *inc),
            TimeControlType::Infinite { .. } => (0, 0, 0), // Not used for infinite time
        };

        let mut moves_played = Vec::new();
        let mut positions = Vec::new();
        let mut position_counts = HashMap::new();

        let initial_board = game.current_position();
        positions.push(initial_board);
        *position_counts.entry(initial_board).or_insert(0) += 1;

        white_engine.new_game().unwrap();
        black_engine.new_game().unwrap();

        let mut move_count = 0;
        while game.result().is_none() {
            let board = game.current_position();
            let engine = match board.side_to_move() {
                chess::Color::White => &mut white_engine,
                chess::Color::Black => &mut black_engine,
            };

            // Make the move based on time control type
            let start_time = std::time::Instant::now();
            let mv = match &args.time_control {
                TimeControlType::Infinite { move_time } => {
                    engine.best_move_infinite(&board.to_string(), *move_time)?
                }
                TimeControlType::TimeControl { .. } => {
                    engine.best_move_timed(&board.to_string(), white_time, black_time, increment)?
                }
            };
            let elapsed = start_time.elapsed().as_millis() as u64;

            // If using a time control, declare a loss on time before applying the move
            if matches!(args.time_control, TimeControlType::TimeControl { .. }) {
                let mover = board.side_to_move();
                let available_time = match mover {
                    chess::Color::White => white_time,
                    chess::Color::Black => black_time,
                };

                if elapsed > available_time {
                    // Encode flagfall as a resignation for correct PGN result
                    match mover {
                        chess::Color::White => {
                            let _ = game.resign(chess::Color::White);
                        }
                        chess::Color::Black => {
                            let _ = game.resign(chess::Color::Black);
                        }
                    }
                    break;
                }
            }

            // Update time for the player who just moved (subtract time used, add increment)
            if matches!(args.time_control, TimeControlType::TimeControl { .. }) {
                match board.side_to_move() {
                    chess::Color::White => {
                        white_time = white_time.saturating_sub(elapsed) + increment;
                    }
                    chess::Color::Black => {
                        black_time = black_time.saturating_sub(elapsed) + increment;
                    }
                }
            }

            game.make_move(mv);
            moves_played.push(mv);

            let new_board = game.current_position();
            positions.push(new_board);
            *position_counts.entry(new_board).or_insert(0) += 1;

            if position_counts[&new_board] >= 2 || move_count > 300 {
                game.declare_draw();
            }

            move_count += 1;
        }

        let result = game.result().ok_or("Game ended without a result")?;

        Ok(GameOutcome {
            opening: args.opening.clone(),
            white_name: self.white.clone(),
            black_name: self.black.clone(),
            result,
            moves: moves_played,
            positions,
        })
    }
}
