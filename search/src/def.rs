use chess::{Board, ChessMove};
use evaluation::{HCE, NNUE};
use std::sync::mpsc::Sender;
use uci::{commands::GoParams, UciOutput};

pub trait Engine {
    fn new(hce: Box<dyn HCE>, nnue: Option<Box<dyn NNUE>>) -> Self;
    fn new_game(&mut self);
    fn set_position(&mut self, board: Board);
    fn search(
        &mut self,
        params: &GoParams,
        output: Option<&Sender<UciOutput>>,
    ) -> Option<(ChessMove, i16)>;
    fn stop(&mut self);
    fn name(&self) -> String;
}
