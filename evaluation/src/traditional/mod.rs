pub mod bonus;
pub mod evaluation;
mod pst;

pub use evaluation::evaluate_board;

use crate::def::Evaluator;
use chess::Board;

pub struct TraditionalEvaluator;

impl Evaluator for TraditionalEvaluator {
    fn name(&self) -> String {
        "Traditional".to_string()
    }

    fn evaluate(
        &mut self,
        board: &Board,
        white_has_castled: bool,
        black_has_castled: bool,
        phase: f32,
    ) -> i16 {
        evaluate_board(board, white_has_castled, black_has_castled, phase)
    }
}
