use std::str::FromStr;

use chess::{Board, ChessMove};

use crate::uci::commands::GoParams;

use super::commands::UciInput;

pub struct Decoder {}

impl Decoder {
    pub fn decode(&self, input: &str) -> UciInput {
        match input {
            "uci" => UciInput::Uci,
            "isready" => UciInput::IsReady,
            "ucinewgame" => UciInput::UciNewGame,

            _ if input.starts_with("position") => self.decode_position(input),
            _ if input.starts_with("go") => self.decode_go(input),
            _ if input.starts_with("stop") => UciInput::Stop,
            _ if input.starts_with("quit") => UciInput::Quit,

            _ => UciInput::Unknown(input.to_string()),
        }
    }

    fn decode_position(&self, input: &str) -> UciInput {
        let mut board = if input.contains("fen") {
            // Extract FEN string (everything between "fen" and "moves" or end of string)
            let fen = input
                .split("fen")
                .nth(1)
                .unwrap()
                .split("moves")
                .next()
                .unwrap()
                .trim();
            Board::from_str(fen).unwrap()
        } else {
            Board::default() // Default to startpos
        };

        // Handle moves if present
        if input.contains("moves") {
            let moves = input
                .split("moves")
                .nth(1)
                .unwrap()
                .trim()
                .split_whitespace();

            for mv in moves {
                let mv = ChessMove::from_san(&board, mv).unwrap();
                board = board.make_move_new(mv);
            }
        }

        UciInput::Position(board)
    }
    fn decode_go(&self, input: &str) -> UciInput {
        UciInput::Go(GoParams {
            ponder: input.contains("ponder"),
            infinite: input.contains("infinite"),
            searchmoves: None, // We'll implement this later
            wtime: extract_numeric_param(input, "wtime"),
            btime: extract_numeric_param(input, "btime"),
            winc: extract_numeric_param(input, "winc"),
            binc: extract_numeric_param(input, "binc"),
            movestogo: extract_numeric_param(input, "movestogo"),
            depth: extract_numeric_param(input, "depth"),
            nodes: extract_numeric_param(input, "nodes"),
            movetime: extract_numeric_param(input, "movetime"),
        })
    }
}

fn extract_numeric_param(input: &str, param: &str) -> Option<u64> {
    input
        .split_whitespace()
        .collect::<Vec<&str>>()
        .windows(2)
        .find(|w| w[0] == param)
        .and_then(|w| w[1].parse().ok())
}
