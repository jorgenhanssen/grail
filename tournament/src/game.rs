use std::{error::Error, path::PathBuf, str::FromStr};

use chess::Board;

use crate::{engine::EngineProcess, outcome::GameOutcome, utils::color_to_index};

#[derive(Debug)]
pub struct GameArgs {
    pub move_time: u64,
    pub start_position_fen: String,
}

pub struct GameRunner {
    white: PathBuf,
    black: PathBuf,
}

impl GameRunner {
    pub fn new(white: PathBuf, black: PathBuf) -> Self {
        Self { white, black }
    }

    pub fn run(&self, args: GameArgs) -> Result<GameOutcome, Box<dyn Error>> {
        use chess::Game;

        let mut moves_played = Vec::new();
        let mut positions = Vec::new();

        let mut game = match Game::from_str(&args.start_position_fen) {
            Ok(game) => game,
            Err(e) => return Err(format!("Invalid FEN: {}", e).into()),
        };

        let mut players = [
            (EngineProcess::new(&self.white)?),
            (EngineProcess::new(&self.black)?),
        ];

        while game.result().is_none() {
            let board = game.current_position();

            if has_threefold_repetition(&positions, &board) {
                game.declare_draw();
            }

            positions.push(board);

            let player = &mut players[color_to_index(board.side_to_move())];
            let mv = player.best_move(&board.to_string(), args.move_time);

            game.make_move(mv);
            moves_played.push(mv);
        }

        let result = game.result().unwrap();

        Ok(GameOutcome {
            white_name: self.white.to_string_lossy().to_string(),
            black_name: self.black.to_string_lossy().to_string(),
            result,
            moves: moves_played,
            positions,
        })
    }
}

#[inline]
fn has_threefold_repetition(positions: &Vec<Board>, current_position: &Board) -> bool {
    positions.iter().filter(|&p| p == current_position).count() >= 3
}
