use chess::Board;

pub trait Evaluator {
    fn name(&self) -> String;
    fn evaluate(&mut self, board: &Board, white_has_castled: bool, black_has_castled: bool) -> i16;
}
