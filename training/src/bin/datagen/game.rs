use crate::args::Args;
use crate::sampler::Sampler;
use cozy_chess::{Board, Move};
use search::{Engine, SearchResult};
use std::collections::HashMap;
use training::dataset::{GameData, GameOutcome, MoveData, Sample};
use uci::commands::GoParams;
use utils::{
    Node, NodeType, flip_eval_perspective, has_check, has_insufficient_material, has_legal_moves,
};

#[derive(Clone, Copy)]
pub struct GameConfig {
    pub scout_nodes: u64,
    pub eval_nodes: u64,
    pub max_opening_imbalance: Option<i16>,
    pub max_game_plies: usize,
}

impl GameConfig {
    pub fn from_args(args: &Args) -> Self {
        Self {
            scout_nodes: args.scout_nodes,
            eval_nodes: args.eval_nodes,
            max_opening_imbalance: args.max_opening_imbalance,
            max_game_plies: args.max_game_plies as usize,
        }
    }
}

/// A self-play game that records positions for training.
pub struct SelfPlayGame {
    board: Board,
    position_counts: HashMap<u64, usize>,
    data: GameData,
    config: GameConfig,
    sampler: Sampler,
}

impl SelfPlayGame {
    pub fn new(board: Board, config: GameConfig, sampler: Sampler) -> Self {
        Self {
            data: GameData {
                start_position: board.clone(),
                moves: Vec::new(),
            },
            board,
            position_counts: HashMap::new(),
            config,
            sampler,
        }
    }

    pub fn play(&mut self, engine: &mut Engine) {
        engine.new_game();

        loop {
            // Cursed long game... no trustworthy outcome, discard the game.
            if self.data.moves.len() >= self.config.max_game_plies {
                self.data.moves.clear();
                return;
            }

            if self.is_terminal() {
                break;
            }

            let node = Node::new(self.board.clone(), NodeType::Pv);
            let static_eval = engine.get_static_eval(&node);

            let Some(mut result) = self.search(engine, self.config.scout_nodes) else {
                break;
            };

            let scout_eval = flip_eval_perspective(
                self.board.side_to_move(),
                result.primary().expect("has lines").score,
            );

            // Scout node is lucky winner and becomes a sample!
            let is_sample = self.sampler.should_sample(&node, static_eval, scout_eval);
            if is_sample {
                let Some(eval_result) = self.search(engine, self.config.eval_nodes) else {
                    break;
                };
                result = eval_result;
            }

            let Some(pv) = result.primary() else {
                break;
            };

            if self.opening_is_too_imbalanced(pv.score) {
                break;
            }

            let sample = if is_sample {
                Some(Sample {
                    score: flip_eval_perspective(self.board.side_to_move(), pv.score),
                    outcome: GameOutcome::Draw,
                })
            } else {
                None
            };

            let chosen = result.select_softmax().expect("has lines");
            let Some(mv) = chosen.best_move() else {
                break;
            };

            self.play_move(mv, sample);
        }
    }

    fn search(&self, engine: &mut Engine, nodes: u64) -> Option<SearchResult> {
        engine.set_position(self.board.clone(), Some(self.history()));

        let params = GoParams {
            soft_nodes: Some(nodes),
            ..Default::default()
        };

        engine.search(&params, None)
    }

    fn opening_is_too_imbalanced(&self, score: i16) -> bool {
        let Some(max) = self.config.max_opening_imbalance else {
            return false;
        };
        self.data.moves.is_empty() && score.abs() > max
    }

    /// The game outcome. Returns None for speculative draws (50-move rule / repetition)
    pub fn outcome(&self) -> Option<GameOutcome> {
        if !has_legal_moves(&self.board) {
            return Some(if has_check(&self.board) {
                GameOutcome::win(!self.board.side_to_move())
            } else {
                GameOutcome::Draw
            });
        }

        if has_insufficient_material(&self.board) {
            return Some(GameOutcome::Draw);
        }

        // During some testing, I found that a winning game can still end in a draw in datagen.
        // Looked like it could shuffle won endgames into draws somehow.
        // Anyways, in these games it is better to have the tb define the outcome.
        None
    }

    /// Play a single move, updating all game state.
    fn play_move(&mut self, mv: Move, sample: Option<Sample>) {
        self.data.moves.push(MoveData { mv, sample });
        self.board.play_unchecked(mv);

        // Track position for repetition detection
        let hash = self.board.hash();
        *self.position_counts.entry(hash).or_insert(0) += 1;
    }

    fn is_terminal(&self) -> bool {
        if !has_legal_moves(&self.board) {
            return true;
        }
        if has_insufficient_material(&self.board) {
            return true;
        }
        if self.board.halfmove_clock() >= 100 {
            return true;
        }
        // Any repetition ends game for training purposes
        let hash = self.board.hash();
        if self.position_counts.get(&hash).copied().unwrap_or(0) >= 2 {
            return true;
        }
        false
    }

    /// Get position history for repetition detection during search.
    fn history(&self) -> ahash::AHashSet<u64> {
        let current_hash = self.board.hash();
        self.position_counts
            .keys()
            .copied()
            .filter(|&hash| hash != current_hash)
            .collect()
    }

    pub fn into_data(self) -> GameData {
        self.data
    }
}
