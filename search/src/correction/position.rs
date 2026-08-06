use cozy_chess::{Board, Color, Square};
use utils::{MINOR_PIECES, NON_PAWN_PIECES, PAWN_PIECES, generate_zobrist_table, zobrist_key};

use config::EngineConfig;

use crate::history::apply_gravity;

// Zobrist tables for correction history indexing.
const PAWN_ZOBRIST: [u64; Square::NUM * Color::NUM * PAWN_PIECES.len()] =
    generate_zobrist_table(0xE2E4_D7D5_E4D5_D8D5);
const MINOR_ZOBRIST: [u64; Square::NUM * Color::NUM * MINOR_PIECES.len()] =
    generate_zobrist_table(0xE2E4_C7C5_D2D4_C5D4);
const NON_PAWN_ZOBRIST: [u64; Square::NUM * NON_PAWN_PIECES.len()] =
    generate_zobrist_table(0xD2D4_D7D5_C2C4_E7E6);

/// Correction indexed by partial Zobrist hashes of the board for different piece types.
#[derive(Clone)]
pub(super) struct PositionCorrection {
    // [color][zobrist_hash].
    pawn: Vec<i16>,
    // [color][zobrist_hash].
    minor: Vec<i16>,
    // [color][zobrist_hash].
    white_nonpawn: Vec<i16>,
    // [color][zobrist_hash].
    black_nonpawn: Vec<i16>,

    table_size: usize,

    pawn_weight: i32,
    minor_weight: i32,
    nonpawn_weight: i32,

    minor_update_weight: i32,
    nonpawn_update_weight: i32,
}

impl PositionCorrection {
    pub(super) fn new(config: &EngineConfig) -> Self {
        let table_size = config.correction_table_size;
        let total_size = Color::NUM * table_size;
        Self {
            pawn: vec![0; total_size],
            minor: vec![0; total_size],
            white_nonpawn: vec![0; total_size],
            black_nonpawn: vec![0; total_size],
            table_size,
            pawn_weight: config.correction_pawn_weight,
            minor_weight: config.correction_minor_weight,
            nonpawn_weight: config.correction_nonpawn_weight,
            minor_update_weight: config.correction_minor_update_weight,
            nonpawn_update_weight: config.correction_nonpawn_update_weight,
        }
    }

    pub(super) fn configure(&mut self, config: &EngineConfig) {
        let new_table_size = config.correction_table_size;
        if self.table_size != new_table_size {
            let total_size = Color::NUM * new_table_size;
            self.pawn = vec![0; total_size];
            self.minor = vec![0; total_size];
            self.white_nonpawn = vec![0; total_size];
            self.black_nonpawn = vec![0; total_size];
            self.table_size = new_table_size;
        }
        self.pawn_weight = config.correction_pawn_weight;
        self.minor_weight = config.correction_minor_weight;
        self.nonpawn_weight = config.correction_nonpawn_weight;
        self.minor_update_weight = config.correction_minor_update_weight;
        self.nonpawn_update_weight = config.correction_nonpawn_update_weight;
    }

    pub(super) fn matches_config(&self, config: &EngineConfig) -> bool {
        self.table_size == config.correction_table_size
            && self.pawn_weight == config.correction_pawn_weight
            && self.minor_weight == config.correction_minor_weight
            && self.nonpawn_weight == config.correction_nonpawn_weight
            && self.minor_update_weight == config.correction_minor_update_weight
            && self.nonpawn_update_weight == config.correction_nonpawn_update_weight
    }

    pub(super) fn reset(&mut self) {
        self.pawn.fill(0);
        self.minor.fill(0);
        self.white_nonpawn.fill(0);
        self.black_nonpawn.fill(0);
    }

    /// Combined correction value for this position across all four tables.
    pub(super) fn weighted_value(&self, board: &Board) -> i32 {
        let (pawn_idx, minor_idx, white_nonpawn_idx, black_nonpawn_idx) = self.get_indices(board);

        let pawn_value = self.pawn[pawn_idx] as i32;
        let minor_value = self.minor[minor_idx] as i32;
        let white_nonpawn_value = self.white_nonpawn[white_nonpawn_idx] as i32;
        let black_nonpawn_value = self.black_nonpawn[black_nonpawn_idx] as i32;

        self.pawn_weight * pawn_value
            + self.minor_weight * minor_value
            + self.nonpawn_weight * (white_nonpawn_value + black_nonpawn_value)
    }

    /// Update each table with a gravity-scaled bonus. The pawn table uses the
    /// raw bonus, the others scale it by their own update weight.
    pub(super) fn apply_bonus(&mut self, board: &Board, bonus: i32, limit: i32) {
        let (pawn_idx, minor_idx, white_nonpawn_idx, black_nonpawn_idx) = self.get_indices(board);

        let pawn_bonus = bonus;
        let minor_bonus = bonus * self.minor_update_weight / 128;
        let nonpawn_bonus = bonus * self.nonpawn_update_weight / 128;

        apply_gravity(&mut self.pawn[pawn_idx], pawn_bonus, limit);
        apply_gravity(&mut self.minor[minor_idx], minor_bonus, limit);
        apply_gravity(
            &mut self.white_nonpawn[white_nonpawn_idx],
            nonpawn_bonus,
            limit,
        );
        apply_gravity(
            &mut self.black_nonpawn[black_nonpawn_idx],
            nonpawn_bonus,
            limit,
        );
    }

    fn get_indices(&self, board: &Board) -> (usize, usize, usize, usize) {
        let color = board.side_to_move();

        let pawn_idx = self.index(color, zobrist_key!(board, PAWN_ZOBRIST, PAWN_PIECES));
        let minor_idx = self.index(color, zobrist_key!(board, MINOR_ZOBRIST, MINOR_PIECES));
        let white_nonpawn_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::White),
        );
        let black_nonpawn_idx = self.index(
            color,
            zobrist_key!(board, NON_PAWN_ZOBRIST, NON_PAWN_PIECES, Color::Black),
        );

        (pawn_idx, minor_idx, white_nonpawn_idx, black_nonpawn_idx)
    }

    fn index(&self, color: Color, key: u64) -> usize {
        let color_idx = color as usize;
        let key_idx = (key as usize) % self.table_size;
        color_idx * self.table_size + key_idx
    }
}
