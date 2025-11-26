use cozy_chess::{Board, Move, Piece, Square};

use crate::EngineConfig;

const CAPTURE_HISTORY_SIZE: usize = Piece::NUM * Square::NUM * Piece::NUM;

#[derive(Clone)]
pub struct CaptureHistory {
    history: Vec<i16>,
    max_value: i32,
    bonus_multiplier: i32,
    malus_multiplier: i32,
}

impl CaptureHistory {
    pub fn new(max_value: i32, bonus_multiplier: i32, malus_multiplier: i32) -> Self {
        Self {
            history: vec![0; CAPTURE_HISTORY_SIZE],
            max_value,
            bonus_multiplier,
            malus_multiplier,
        }
    }

    pub fn configure(&mut self, config: &EngineConfig) {
        self.max_value = config.capture_history_max_value.value;
        self.bonus_multiplier = config.capture_history_bonus_multiplier.value;
        self.malus_multiplier = config.capture_history_malus_multiplier.value;
        self.reset();
    }

    pub fn matches_config(&self, config: &EngineConfig) -> bool {
        self.max_value == config.capture_history_max_value.value
            && self.bonus_multiplier == config.capture_history_bonus_multiplier.value
            && self.malus_multiplier == config.capture_history_malus_multiplier.value
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.history.fill(0);
    }

    #[inline(always)]
    pub fn get(&self, board: &Board, mv: Move) -> i16 {
        let attacker = board.piece_on(mv.from).unwrap();
        let victim = board.piece_on(mv.to).unwrap();
        self.history[Self::index(attacker, mv.to, victim)]
    }

    #[inline(always)]
    pub fn update_capture(&mut self, board: &Board, mv: Move, delta: i32) {
        let attacker = board.piece_on(mv.from).unwrap();
        let victim = match board.piece_on(mv.to) {
            Some(v) => v,
            None => return, // Not a capture
        };

        let idx = Self::index(attacker, mv.to, victim);
        let entry = &mut self.history[idx];
        let h = *entry as i32;
        let b = delta.clamp(-self.max_value, self.max_value);
        let new = h + b - ((h * b.abs()) / self.max_value);
        *entry = new.clamp(-self.max_value, self.max_value) as i16;
    }

    #[inline(always)]
    fn index(attacker: Piece, to: Square, victim: Piece) -> usize {
        let attacker_idx = attacker as usize;
        let to_idx = to as usize;
        let victim_idx = victim as usize;
        attacker_idx * Square::NUM * Piece::NUM + to_idx * Piece::NUM + victim_idx
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        self.bonus_multiplier * remaining_depth as i32
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        -self.malus_multiplier * remaining_depth as i32
    }
}
