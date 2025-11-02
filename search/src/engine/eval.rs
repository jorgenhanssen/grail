use utils::Position;

use super::Engine;

impl Engine {
    #[inline(always)]
    pub(super) fn eval(&mut self, position: &Position, phase: f32) -> i16 {
        let mut score = if let Some(nnue) = &mut self.nnue {
            nnue.evaluate(position.board)
        } else {
            self.hce.evaluate(position, phase)
        };

        score = self.apply_penalties(score, phase);

        score
    }

    #[inline(always)]
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

    #[inline(always)]
    fn piece_repetition_penalty(&self) -> i16 {
        let base_penalty = self.config.piece_repetition_base_penalty.value;
        self.search_stack.piece_repetition_penalty(base_penalty)
    }
}
