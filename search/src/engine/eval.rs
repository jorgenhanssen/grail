use utils::{Node, cap_eval_by_material, flip_eval_perspective};

use super::Engine;

impl Engine {
    /// Get the static evaluation from the perspective of the side to move.
    pub(super) fn static_eval(&mut self, node: &Node) -> i16 {
        let phase = node.game_phase();

        let mut score = if self.config.nnue.value && self.nnue.is_some() {
            self.nnue.as_mut().unwrap().evaluate(node)
        } else {
            self.hce.evaluate(node, phase)
        };

        score = self.apply_penalties(score, phase);
        score = cap_eval_by_material(node.board(), score);

        flip_eval_perspective(node.side_to_move(), score)
    }

    /// Get the static evaluation with correction history applied.
    pub(super) fn corrected_static_eval(&mut self, node: &Node) -> i16 {
        let eval = self.static_eval(node);
        self.correction_history.adjust(node.board(), eval)
    }

    fn apply_penalties(&self, score: i16, phase: f32) -> i16 {
        let mut adjusted_score = score;

        // Piece repetition penalty (opening/middlegame)
        let min_phase = self.config.piece_repetition_min_phase.value / 100.0;
        if phase > min_phase {
            let normalized_phase = (phase - min_phase) / (1.0 - min_phase);
            let penalty = self.piece_repetition_penalty();
            adjusted_score -= ((penalty as f32) * normalized_phase).round() as i16;
        }

        adjusted_score
    }

    fn piece_repetition_penalty(&self) -> i16 {
        let base_penalty = self.config.piece_repetition_base_penalty.value;
        self.search_stack.piece_repetition_penalty(base_penalty)
    }

    /// Returns a small random value for draws to avoid draw blindness.
    /// Based on Stockfish's approach: VALUE_DRAW - 1 + (nodes & 0x2)
    /// Returns -1 or +1 to break symmetry and prevent repetitive play.
    pub(super) fn draw_value(&self) -> i16 {
        -1 + (self.nodes & 0x2) as i16
    }

    /// Check if the position is a forced draw (fifty-move rule or repetition).
    pub(super) fn is_forced_draw(&self, node: &Node) -> bool {
        node.is_fifty_move_draw() || self.search_stack.is_repetition(&self.game_history)
    }
}
