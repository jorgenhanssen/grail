use std::collections::HashMap;

use chess::Game;
use search::{Engine, NegamaxEngine};

pub struct Arena {
    engines: Vec<NegamaxEngine>,
}

impl Arena {
    pub fn new(engines: Vec<NegamaxEngine>) -> Self {
        Self { engines }
    }

    pub fn run_tournament(&mut self, depth: u8) -> HashMap<String, i64> {
        let mut scores = HashMap::new();
        for engine in &self.engines {
            scores.insert(engine.name(), 0);
        }

        let num_engines = self.engines.len();

        let mut matchups = Vec::new();
        for white_index in 0..num_engines {
            for black_index in 0..num_engines {
                if white_index == black_index {
                    continue;
                }
                matchups.push((white_index, black_index));
            }
        }

        for (white_idx, black_idx) in matchups {
            let (white, black) = get_two_mut(&mut self.engines, white_idx, black_idx);
            let result = Arena::play_game(white, black, depth);

            match result {
                chess::GameResult::WhiteCheckmates => {
                    scores.entry(white.name()).and_modify(|w| *w += 1);
                    scores.entry(black.name()).and_modify(|w| *w -= 1);
                }
                chess::GameResult::BlackCheckmates => {
                    scores.entry(black.name()).and_modify(|w| *w += 1);
                    scores.entry(white.name()).and_modify(|w| *w -= 1);
                }
                _ => continue,
            };
        }

        return scores;
    }

    fn play_game(
        white: &mut NegamaxEngine,
        black: &mut NegamaxEngine,
        depth: u8,
    ) -> chess::GameResult {
        let mut game = Game::new();
        let mut num_moves = 0;
        let mut position_counts: HashMap<u64, u32> = HashMap::new();

        while game.result().is_none() {
            let board = game.current_position();
            let current_hash = board.get_hash();
            *position_counts.entry(current_hash).or_insert(0) += 1;

            if position_counts.get(&current_hash).unwrap() >= &3 {
                log::info!(
                    "{} vs {} = Draw (repetition): {:?}",
                    white.name(),
                    black.name(),
                    board.to_string()
                );
                return chess::GameResult::DrawAccepted;
            }
            if num_moves > 100_000 {
                log::info!(
                    "{} vs {} = Draw (too many moves): {:?}",
                    white.name(),
                    black.name(),
                    board.to_string()
                );
                return chess::GameResult::DrawAccepted;
            }

            let player = match game.side_to_move() {
                chess::Color::White => &mut *white,
                chess::Color::Black => &mut *black,
            };

            player.set_position(board);
            player.init_search();

            let (mv, _) = player.search_root(depth);
            game.make_move(mv.unwrap());

            num_moves += 1;
        }

        let winner = match game.result().unwrap() {
            chess::GameResult::WhiteCheckmates => white.name(),
            chess::GameResult::BlackCheckmates => black.name(),
            _ => unreachable!(),
        };

        log::info!(
            "{} vs {} = {}: {:?}",
            white.name(),
            black.name(),
            winner,
            game.current_position().to_string()
        );

        game.result().unwrap()
    }
}

fn get_two_mut<T>(vec: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j, "Indices must differ!");
    if i < j {
        let (left, right) = vec.split_at_mut(j);
        (&mut left[i], &mut right[0])
    } else {
        let (left, right) = vec.split_at_mut(i);
        (&mut right[0], &mut left[j])
    }
}
