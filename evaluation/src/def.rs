use chess::Board;

pub trait Evaluator {
    fn name(&self) -> String;
    fn evaluate(&mut self, board: &Board) -> i32;
}
