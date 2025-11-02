use chess::{BitBoard, Board, ChessMove, Color, Square, EMPTY, NUM_COLORS, NUM_SQUARES};

use crate::EngineConfig;

const MAX_DEPTH: usize = 100;
// [color][is_threatened][from][to] (similar to Black Marlin)
const HISTORY_SIZE: usize = NUM_COLORS * 2 * NUM_SQUARES * NUM_SQUARES;

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

    #[inline(always)]
    pub fn reset(&mut self) {
        self.history.fill(0);
    }

    #[inline(always)]
    pub fn get(&self, color: Color, source: Square, dest: Square, threats: BitBoard) -> i16 {
        let is_threatened = threats & BitBoard::from_square(source) != EMPTY;
        self.history[Self::index(color, is_threatened, source, dest)]
    }

    #[inline(always)]
    fn update_move(
        &mut self,
        c: Color,
        is_threatened: bool,
        source: Square,
        dest: Square,
        bonus: i32,
    ) {
        let idx = Self::index(c, is_threatened, source, dest);
        let entry = &mut self.history[idx];

        let h = *entry as i32;
        let b = bonus.clamp(-(self.max_history), self.max_history);

        // History gravity formula
        let new = h + b - ((h * b.abs()) / self.max_history);

        *entry = new.clamp(-(self.max_history), self.max_history) as i16;
    }

    #[inline(always)]
    pub fn update(&mut self, board: &Board, mv: ChessMove, delta: i32, threats: BitBoard) {
        let color = board.side_to_move();
        let source = mv.get_source();
        let dest = mv.get_dest();
        let is_threatened = threats & BitBoard::from_square(source) != EMPTY;
        self.update_move(color, is_threatened, source, dest, delta);
    }

    #[inline(always)]
    fn index(color: Color, is_threatened: bool, source: Square, dest: Square) -> usize {
        let color_idx = color.to_index();
        let threat_idx = is_threatened as usize;
        let source_idx = source.to_index();
        let dest_idx = dest.to_index();

        let color_stride = 2 * NUM_SQUARES * NUM_SQUARES;
        let threat_stride = NUM_SQUARES * NUM_SQUARES;
        let source_stride = NUM_SQUARES;

        color_idx * color_stride
            + threat_idx * threat_stride
            + source_idx * source_stride
            + dest_idx
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn maybe_reduce_or_prune(
        &self,
        board: &Board,
        mv: ChessMove,
        depth: u8,
        max_depth: u8,
        remaining_depth: u8,
        in_check: bool,
        is_tactical: bool,
        is_pv_move: bool,
        move_index: i32,
        is_improving: bool,
        reduction: &mut u8,
        threats: BitBoard,
    ) -> bool {
        if !(remaining_depth > 0
            && !in_check
            && !is_tactical
            && !is_pv_move
            && move_index >= self.min_move_index)
        {
            return false;
        }

        let color = board.side_to_move();
        let source = mv.get_source();
        let dest = mv.get_dest();
        let hist_score = self.get(color, source, dest, threats);

        if hist_score < self.reduction_threshold {
            // Only apply additional reductions/pruning when position isn't improving
            if !is_improving {
                *reduction = reduction.saturating_add(1);
            }

            let projected_child_max = max_depth.saturating_sub(*reduction);
            if !is_improving
                && hist_score < self.prune_threshold
                && projected_child_max <= depth + 1
            {
                return true; // prune
            }
        }

        false
    }

    #[inline(always)]
    pub fn get_bonus(&self, remaining_depth: u8) -> i32 {
        self.bonus_multiplier * remaining_depth.min(MAX_DEPTH as u8) as i32
    }

    #[inline(always)]
    pub fn get_malus(&self, remaining_depth: u8) -> i32 {
        -self.malus_multiplier * remaining_depth.min(MAX_DEPTH as u8) as i32
    }
}
