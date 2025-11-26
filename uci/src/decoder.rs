use std::str::FromStr;

use ahash::AHashSet;
use cozy_chess::{util::parse_uci_move, Board};

use super::commands::{GoParams, UciInput};

pub struct Decoder {}

impl Decoder {
    pub fn decode(&self, input: &str) -> UciInput {
        match input {
            "uci" => UciInput::Uci,
            "isready" => UciInput::IsReady,
            "ucinewgame" => UciInput::UciNewGame,

            _ if input.starts_with("position") => self.decode_position(input),
            _ if input.starts_with("go") => self.decode_go(input),
            _ if input.starts_with("setoption") => self.decode_setoption(input),
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

        // Track positions seen in the game (not including the current position)
        // The current position will be the search root (included in search stack)
        let mut game_history = AHashSet::new();

        // Parse and apply moves
        if input.contains("moves") {
            let move_strings = input
                .split("moves")
                .nth(1)
                .unwrap()
                .trim()
                .split_whitespace();

            for mv_str in move_strings {
                game_history.insert(board.hash());

                // parse_uci_move needed since cozy-chess uses "king captures rook" notation internally
                if let Ok(mv) = parse_uci_move(&board, mv_str) {
                    board.play(mv);
                }
            }
        }

        UciInput::Position {
            board,
            game_history,
        }
    }

    fn decode_setoption(&self, input: &str) -> UciInput {
        // Parse setoption name <name> value <value>
        if let Some(name_start) = input.find("name ") {
            if let Some(value_start) = input.find(" value ") {
                let name_part = &input[name_start + 5..value_start];
                let value_part = &input[value_start + 7..];

                return UciInput::SetOption {
                    name: name_part.trim().to_string(),
                    value: value_part.trim().to_string(),
                };
            }
        }

        UciInput::Unknown(input.to_string())
    }

    fn decode_go(&self, input: &str) -> UciInput {
        UciInput::Go(GoParams {
            infinite: input.contains("infinite"),
            search_moves: None, // We'll implement this later
            wtime: extract_numeric_param(input, "wtime"),
            btime: extract_numeric_param(input, "btime"),
            winc: extract_numeric_param(input, "winc"),
            binc: extract_numeric_param(input, "binc"),
            moves_to_go: extract_numeric_param(input, "movestogo"),

            // TODO: Consider error handling
            depth: extract_numeric_param(input, "depth").map(|d| d as u8),
            move_time: extract_numeric_param(input, "movetime"),
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
