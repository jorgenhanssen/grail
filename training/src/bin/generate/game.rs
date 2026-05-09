use crate::samples::{GameOutcome, Sample};
use cozy_chess::{Board, Color, Move};
use pyrrhic_rs::{TableBases, WdlProbeResult};
use rand::Rng;
use search::{CozyAdapter, Engine, PvLine, SearchResult};
use std::collections::HashMap;
use std::str::FromStr;
use uci::commands::GoParams;
use utils::{flip_eval_perspective, has_check, has_insufficient_material, has_legal_moves};

/// A self-play game that generates training samples.
///
/// Uses MultiPV search at decision points and teleports along chosen PV lines
/// to reduce sample correlation and increase game diversity.
pub struct SelfPlayGame {
    board: Board,
    game_id: usize,
    position_counts: HashMap<u64, usize>,
    positions: Vec<(String, i16, Move)>, // FEN, eval, best move
    depth: u8,
    tablebases: Option<TableBases<CozyAdapter>>,
}

impl SelfPlayGame {
    pub fn new(
        game_id: usize,
        opening_fen: &str,
        depth: u8,
        tablebases: Option<TableBases<CozyAdapter>>,
    ) -> Self {
        let board = Board::from_str(opening_fen).unwrap();

        Self {
            board,
            game_id,
            position_counts: HashMap::new(),
            positions: Vec::new(),
            depth,
            tablebases,
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

            let Some(result) = self.search(engine) else {
                break;
            };
            let Some(pv) = result.primary() else {
                break;
            };

            if let Some(mv) = pv.best_move() {
                self.record_position(pv.score, mv);
            }

            let chosen_pv = result.select_softmax().expect("has lines");
            self.teleport(chosen_pv, engine);
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

    fn record_position(&mut self, eval: i16, best_move: Move) {
        let white_score = flip_eval_perspective(self.board.side_to_move(), eval);
        self.positions
            .push((format!("{}", self.board), white_score, best_move));
    }

    fn outcome(&self) -> GameOutcome {
        if !has_legal_moves(&self.board) && has_check(&self.board) {
            return if self.board.side_to_move() == Color::White {
                GameOutcome::Black
            } else {
                GameOutcome::White
            };
        }

        GameOutcome::Draw
    }

    /// Teleport along a PV line by playing moves without searching.
    ///
    /// Picks a random teleport length from 1 to depth, then plays that many
    /// moves from the PV. If the PV is shorter than the teleport distance
    /// (e.g. TB positions where the PV is only 1 move), continues teleporting
    /// by searching for the best move at each step.
    fn teleport(&mut self, pv: &PvLine, engine: &mut Engine) {
        if pv.line.is_empty() {
            return;
        }

        let mut rng = rand::rng();
        let teleport_len = rng.random_range(1..=self.depth as usize);

        let mut steps = 0;

        for mv in pv.line.iter().take(teleport_len) {
            self.play_move(*mv);
            steps += 1;
            if self.is_terminal() {
                return;
            }
        }

        while steps < teleport_len {
            let Some(result) = self.search(engine) else {
                return;
            };
            let Some(mv) = result.primary().and_then(PvLine::best_move) else {
                return;
            };
            self.play_move(mv);
            steps += 1;
            if self.is_terminal() {
                return;
            }
        }
    }

    /// Play a single move, updating all game state.
    fn play_move(&mut self, mv: Move) {
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
        if self.position_counts.get(&hash).copied().unwrap_or(0) >= 2 {
            return true;
        }
        // Adjudicate theoretical draws via tablebases (wins/losses play on naturally)
        if let Some(tb) = self.tablebases.as_ref() {
            if let Some(
                WdlProbeResult::Draw | WdlProbeResult::CursedWin | WdlProbeResult::BlessedLoss,
            ) = search::tablebase::probe_wdl(tb, &self.board)
            {
                return true;
            }
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

    pub fn get_samples(&mut self) -> (Vec<Sample>, Vec<i16>) {
        let outcome = self.outcome();
        let (samples, scores): (Vec<_>, Vec<_>) = self
            .positions
            .drain(..)
            .map(|(fen, score, best_move)| {
                let sample = Sample {
                    fen,
                    score,
                    game_id: self.game_id,
                    best_move,
                    outcome,
                };
                (sample, score)
            })
            .unzip();
        (samples, scores)
    }
}
