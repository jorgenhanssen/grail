use crate::EngineConfig;
use chess::{Board, ChessMove};
use evaluation::Evaluator;
use std::sync::mpsc::Sender;
use uci::{commands::GoParams, UciOutput};

pub trait Engine {
    fn new(evaluator: Box<dyn Evaluator>, config: &EngineConfig) -> Self;
    fn configure(&mut self, config: &EngineConfig, init: bool);
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
