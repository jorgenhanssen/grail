use crate::uci::{commands::GoParams, UciOutput};
use chess::{Board, ChessMove};
use std::sync::mpsc::Sender;

pub trait Engine {
    fn set_position(&mut self, board: Board);
    fn search(&mut self, params: &GoParams, output: &Sender<UciOutput>) -> ChessMove;
    fn stop(&mut self);
}
