pub mod evaluation;
mod pst;
pub mod values;

pub use evaluation::evaluate_board;
pub use values::piece_value;

use crate::def::Evaluator;
use chess::Board;

pub struct TraditionalEvaluator;

impl Evaluator for TraditionalEvaluator {
    fn evaluate(&self, board: &Board) -> f32 {
        evaluate_board(board)
    }
}
