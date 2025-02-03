use chess::Board;

pub trait Evaluator {
    fn name(&self) -> String;
    fn evaluate(&self, board: &Board) -> f32;
}
