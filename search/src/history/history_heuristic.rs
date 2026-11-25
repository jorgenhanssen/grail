use cozy_chess::{Color, Move, Square};
use utils::Node;

use config::EngineConfig;

use super::utils::apply_gravity;
use crate::MAX_DEPTH;

const ATTACK_SIZE: usize = 4; // [from attacked][to attacked]
const HISTORY_SIZE: usize = Color::NUM * Square::NUM * Square::NUM * ATTACK_SIZE;

/// Scores quiet moves based on search success.
/// Indexed by [color][from][to][from attacked][to attacked].
/// Moves causing beta cutoffs get bonuses and moves searched before it get malus.
///
/// <https://www.chessprogramming.org/History_Heuristic>
#[derive(Clone)]
pub struct HistoryHeuristic {
    history: Vec<i16>,

    max_history: i32,

    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl HistoryHeuristic {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            history: vec![0; HISTORY_SIZE],
            max_history: config.history_max_value,
            bonus_multiplier: config.history_bonus_multiplier,
            malus_multiplier: config.history_malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_history = config.history_max_value;

        self.bonus_multiplier = config.history_bonus_multiplier;
        self.malus_multiplier = config.history_malus_multiplier;

        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_history == config.history_max_value
            && self.bonus_multiplier == config.history_bonus_multiplier
            && self.malus_multiplier == config.history_malus_multiplier
    }

    pub fn reset(&mut self) {
        self.history.fill(0);
    }

    pub fn get(&self, node: &Node, mv: Move) -> i16 {
        self.history[Self::index(node, mv)]
    }

    pub fn update(&mut self, node: &Node, mv: Move, delta: i32) {
        let idx = Self::index(node, mv);
        apply_gravity(&mut self.history[idx], delta, self.max_history);
    }

    fn index(node: &Node, mv: Move) -> usize {
        let color = node.side_to_move();
        let enemy_attacks = node.attacks_for(!color);

        let color_idx = color as usize;
        let source_idx = mv.from as usize;
        let dest_idx = mv.to as usize;

        let color_stride = Square::NUM * Square::NUM;
        let source_stride = Square::NUM;

        let from_attacked = enemy_attacks.has(mv.from) as usize;
        let to_attacked = enemy_attacks.has(mv.to) as usize;

        let move_idx = color_idx * color_stride + source_idx * source_stride + dest_idx;
        let attack_idx = from_attacked * 2 + to_attacked;

        move_idx * ATTACK_SIZE + attack_idx
    }

    pub fn get_bonus(&self, depth: u8) -> i32 {
        self.bonus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }

    pub fn get_malus(&self, depth: u8) -> i32 {
        -self.malus_multiplier * depth.min(MAX_DEPTH as u8) as i32
    }
}
