use cozy_chess::{BitBoard, Board, Color, Move, Square};

use super::utils::apply_gravity;
use crate::{EngineConfig, MAX_DEPTH};

const HISTORY_SIZE: usize = Color::NUM * 2 * Square::NUM * Square::NUM;

/// Scores quiet moves based on search success. Indexed by [color][is_threatened][from][to].
/// Moves causing beta cutoffs get bonuses; moves searched before a cutoff get malus.
///
/// <https://www.chessprogramming.org/History_Heuristic>
#[derive(Clone)]
pub struct HistoryHeuristic {
    history: Vec<i16>,

    max_history: i32,
    reduction_threshold: i16,
    prune_threshold: i16,
    min_move_index: i32,

    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl HistoryHeuristic {
    pub fn new(
        max_history: i32,
        reduction_threshold: i16,
        prune_threshold: i16,
        min_move_index: i32,
        bonus_multiplier: i32,
        malus_multiplier: i32,
    ) -> Self {
        Self {
            history: vec![0; HISTORY_SIZE],

            max_history,
            reduction_threshold,
            prune_threshold,
            min_move_index,

            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_history = config.history_max_value.value;
        self.reduction_threshold = config.history_reduction_threshold.value;
        self.prune_threshold = config.history_prune_threshold.value;
        self.min_move_index = config.history_min_move_index.value;

        self.bonus_multiplier = config.history_bonus_multiplier.value;
        self.malus_multiplier = config.history_malus_multiplier.value;

        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_history == config.history_max_value.value
            && self.reduction_threshold == config.history_reduction_threshold.value
            && self.prune_threshold == config.history_prune_threshold.value
            && self.min_move_index == config.history_min_move_index.value
            && self.bonus_multiplier == config.history_bonus_multiplier.value
            && self.malus_multiplier == config.history_malus_multiplier.value
    }

    pub fn reset(&mut self) {
        self.history.fill(0);
    }

    pub fn get(&self, color: Color, source: Square, dest: Square, threats: BitBoard) -> i16 {
        let is_threatened = threats.has(source);
        self.history[Self::index(color, is_threatened, source, dest)]
    }

    fn update_move(
        &mut self,
        c: Color,
        is_threatened: bool,
        source: Square,
        dest: Square,
        bonus: i32,
    ) {
        let idx = Self::index(c, is_threatened, source, dest);
        apply_gravity(&mut self.history[idx], bonus, self.max_history);
    }

    pub fn update(&mut self, board: &Board, mv: Move, delta: i32, threats: BitBoard) {
        let color = board.side_to_move();
        let source = mv.from;
        let dest = mv.to;
        let is_threatened = threats.has(source);
        self.update_move(color, is_threatened, source, dest, delta);
    }

    fn index(color: Color, is_threatened: bool, source: Square, dest: Square) -> usize {
        let color_idx = color as usize;
        let threat_idx = is_threatened as usize;
        let source_idx = source as usize;
        let dest_idx = dest as usize;

        let color_stride = 2 * Square::NUM * Square::NUM;
        let threat_stride = Square::NUM * Square::NUM;
        let source_stride = Square::NUM;

        color_idx * color_stride
            + threat_idx * threat_stride
            + source_idx * source_stride
            + dest_idx
    }

    pub fn reduction_threshold(&self) -> i16 {
        self.reduction_threshold
    }

    pub fn prune_threshold(&self) -> i16 {
        self.prune_threshold
    }

    pub fn get_bonus(&self, depth: u8) -> i32 {
        self.bonus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_malus(&self, depth: u8) -> i32 {
        -self.malus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }
}
