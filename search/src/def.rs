use chess::{Board, ChessMove};
use evaluation::Evaluator;
use std::sync::mpsc::Sender;
use uci::{commands::GoParams, UciOutput};

pub trait Engine {
    fn new(evaluator: Box<dyn Evaluator>) -> Self;
    fn set_position(&mut self, board: Board);
    fn search(&mut self, params: &GoParams, output: &Sender<UciOutput>) -> Option<ChessMove>;
    fn stop(&mut self);
    fn name(&self) -> String;
}
