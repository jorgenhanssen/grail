use cozy_chess::Board;
use utils::Position;

pub trait HCE {
    fn name(&self) -> String;
    fn evaluate(&mut self, position: &Position, phase: f32) -> i16;
}

pub trait NNUE {
    fn name(&self) -> String;
    fn evaluate(&mut self, board: &Board) -> i16;
}
