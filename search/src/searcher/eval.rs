use utils::{Node, cap_eval_by_material, flip_eval_perspective};

use super::Searcher;

impl Searcher {
    /// Get the static evaluation from the perspective of the side to move.
    pub(super) fn static_eval(&mut self, node: &Node) -> i16 {
        let mut score = self.evaluator.evaluate(node);

        score = cap_eval_by_material(node.board(), score);

        flip_eval_perspective(node.side_to_move(), score)
    }

    /// Get the static evaluation with correction history applied.
    pub(super) fn corrected_static_eval(&mut self, node: &Node) -> i16 {
        let eval = self.static_eval(node);
        self.shared.correction().adjust(node.board(), eval)
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
