use cozy_chess::Board;
use nnue::network::CP_BOUND;
use rand::Rng;
use search::{Engine, PvLine, SearchResult};
use std::collections::HashMap;
use std::str::FromStr;
use uci::commands::GoParams;
use utils::{flip_eval_perspective, has_insufficient_material, has_legal_moves};

/// A self-play game that generates training samples (eval distillation).
///
/// Uses MultiPV search at decision points and teleports along chosen PV lines
/// to reduce sample correlation and increase game diversity.
pub struct SelfPlayGame {
    board: Board,
    game_id: usize,
    position_counts: HashMap<u64, usize>,
    samples: Vec<(String, i16, String)>,
    depth: u8,
}

impl SelfPlayGame {
    pub fn new(game_id: usize, opening_fen: &str, depth: u8) -> Self {
        let board = Board::from_str(opening_fen).unwrap();

        Self {
            board,
            game_id,
            position_counts: HashMap::new(),
            samples: Vec::new(),
            depth,
        }
    }

    /// Play the game using MultiPV search and teleporting.
    ///
    /// At each decision point: search, record sample, select PV via softmax,
    /// then teleport along the chosen PV.
    pub fn play(&mut self, engine: &mut Engine) {
        engine.new_game();

        loop {
            if self.is_terminal() {
                break;
            }

            let result = match self.search(engine) {
                Some(r) => r,
                None => break,
            };

            self.record_sample(&result);

            let chosen_pv = result.select_softmax().expect("has lines");
            self.teleport(chosen_pv);
        }
    }

    fn search(&self, engine: &mut Engine) -> Option<SearchResult> {
        engine.set_position(self.board.clone(), Some(self.history()));

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        engine.search(&params, None)
    }

    fn record_sample(&mut self, result: &SearchResult) {
        let pv = match result.primary() {
            Some(pv) => pv,
            None => return,
        };

        let best_move = match pv.best_move() {
            Some(mv) => mv,
            None => return,
        };

        // Early testing showed that focusing the network on
        // less extreme scores resulted in better generalization.
        if pv.score.abs() >= CP_BOUND {
            return;
        }

        let white_score = flip_eval_perspective(self.board.side_to_move(), pv.score);
        self.samples.push((
            format!("{}", self.board),
            white_score,
            format!("{}", best_move),
        ));
    }

    /// Teleport along a PV line by playing moves without searching.
    ///
    /// Picks a random teleport length from 1 to pv.len() (capped by depth),
    /// then plays that many moves from the PV.
    fn teleport(&mut self, pv: &PvLine) {
        if pv.line.is_empty() {
            return;
        }

        let mut rng = rand::rng();
        let max_len = pv.line.len().min(self.depth as usize);
        let teleport_len = rng.random_range(1..=max_len);

        for mv in pv.line.iter().take(teleport_len) {
            self.play_move(*mv);
            if self.is_terminal() {
                break;
            }
        }
    }

    /// Play a single move, updating all game state.
    fn play_move(&mut self, mv: cozy_chess::Move) {
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
        // Any repetition ends game for training purposes
        let hash = self.board.hash();
        self.position_counts.get(&hash).copied().unwrap_or(0) >= 2
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

    pub fn drain_samples(&mut self) -> (Vec<(String, i16, String, usize)>, Vec<i16>) {
        let (samples, scores): (Vec<_>, Vec<_>) = self
            .samples
            .drain(..)
            .map(|(fen, score, mv)| ((fen, score, mv, self.game_id), score))
            .unzip();
        (samples, scores)
    }
}
