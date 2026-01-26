use cozy_chess::Board;
use nnue::network::CP_BOUND;
use rand::Rng;
use search::{Engine, PvLine, SearchResult};
use std::collections::HashMap;
use std::str::FromStr;
use uci::commands::GoParams;
use utils::{flip_eval_perspective, has_insufficient_material, has_legal_moves};

/// Number of PV lines to search at each decision point.
const PV_LINES: u8 = 3;

/// A self-play game that generates training samples: (FEN, score, game_id) tuples.
///
/// Uses MultiPV search at decision points and teleports along chosen PV lines
/// to reduce sample correlation and increase game diversity.
pub struct SelfPlayGame {
    board: Board,
    game_id: usize,
    ply_count: usize,
    position_counts: HashMap<u64, usize>,
    samples: Vec<(String, i16)>,
    depth: u8,
}

impl SelfPlayGame {
    pub fn new(game_id: usize, opening_fen: &str, depth: u8) -> Self {
        let board = Board::from_str(opening_fen).unwrap();

        Self {
            board,
            game_id,
            ply_count: 0,
            position_counts: HashMap::new(),
            samples: Vec::new(),
            depth,
        }
    }

    /// Play the game using MultiPV search and teleporting.
    ///
    /// At each decision point:
    /// 1. Run MultiPV search
    /// 2. Record sample (position + score)
    /// 3. Select a PV line via softmax over scores
    /// 4. Teleport along the chosen PV (play multiple moves without searching)
    pub fn play(&mut self, engine: &mut Engine) {
        engine.new_game();

        loop {
            if self.is_terminal() {
                break;
            }

            // Search with MultiPV
            let result = match self.search(engine) {
                Some(r) => r,
                None => break,
            };

            // Get the primary (best) score for this position
            let primary = match result.primary() {
                Some(pv) => pv,
                None => break,
            };

            // Skip near-mate positions
            if primary.score.abs() >= CP_BOUND {
                break;
            }

            // Record sample at this decision point
            self.record_sample(primary.score);

            // Select PV line using softmax over scores
            let chosen_pv = result.select_softmax().unwrap();

            // Teleport along the chosen PV
            self.teleport(chosen_pv);
        }
    }

    /// Run MultiPV search at the current position.
    fn search(&self, engine: &mut Engine) -> Option<SearchResult> {
        engine.set_position(self.board.clone(), Some(self.history()));

        let params = GoParams {
            depth: Some(self.depth),
            ..Default::default()
        };

        engine.search(&params, None)
    }

    /// Record a sample at the current position.
    fn record_sample(&mut self, engine_score: i16) {
        // Engine score is from STM perspective; flip to white's perspective for training
        let white_score = flip_eval_perspective(self.board.side_to_move(), engine_score);
        self.samples.push((format!("{}", self.board), white_score));
    }

    /// Teleport along a PV line by playing moves without searching.
    ///
    /// Picks a random teleport length from 1 to pv.len() (capped by depth),
    /// then plays that many moves from the PV.
    fn teleport(&mut self, pv: &PvLine) {
        if pv.line.is_empty() {
            return;
        }

        let mut rng = rand::thread_rng();
        let max_len = pv.line.len().min(self.depth as usize);
        let teleport_len = rng.gen_range(1..=max_len);

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
        self.ply_count += 1;

        // Track position for repetition detection
        let hash = self.board.hash();
        *self.position_counts.entry(hash).or_insert(0) += 1;
    }

    /// Check if the game has reached a terminal state.
    fn is_terminal(&self) -> bool {
        // No legal moves (checkmate or stalemate)
        if !has_legal_moves(&self.board) {
            return true;
        }

        // Insufficient material
        if has_insufficient_material(&self.board) {
            return true;
        }

        // Repetition (any repeat ends game for training purposes)
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

    /// Drain samples from this game.
    pub fn drain_samples(&mut self) -> (Vec<(String, i16, usize)>, Vec<i16>) {
        let (samples, scores): (Vec<_>, Vec<_>) = self
            .samples
            .drain(..)
            .map(|(fen, score)| ((fen, score, self.game_id), score))
            .unzip();
        (samples, scores)
    }

    /// Returns the number of PV lines to search.
    pub fn pv_lines() -> u8 {
        PV_LINES
    }
}
