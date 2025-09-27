use chess::Board;

pub trait HCE {
    fn name(&self) -> String;
    fn evaluate(&mut self, board: &Board, phase: f32) -> i16;
}

pub trait NNUE {
    fn name(&self) -> String;
    fn evaluate(&mut self, board: &Board) -> i16;
}
