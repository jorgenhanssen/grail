use std::str::FromStr;

use ahash::AHashSet;
use cozy_chess::{util::parse_uci_move, Board};

use super::commands::{GoParams, UciInput};

pub struct Decoder;

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self
    }

    pub fn decode(&self, input: &str) -> UciInput {
        match input {
            "uci" => UciInput::Uci,
            "isready" => UciInput::IsReady,
            "ucinewgame" => UciInput::UciNewGame,

            _ if input.starts_with("debug") => UciInput::Debug(input.ends_with(" on")),
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
            let move_strings = input.split("moves").nth(1).unwrap().split_whitespace();

            for mv_str in move_strings {
                game_history.insert(board.hash());

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
        // Parse: setoption name <name> [value <value>]
        // Value is optional (button-type options have no value)
        //
        // TODO: Consider returning Result or adding InvalidCommand variant
        // instead of using empty name to signal malformed commands.
        let Some(rest) = input.strip_prefix("setoption name ") else {
            // Missing "name" keyword - return empty name for error handling
            return UciInput::SetOption {
                name: String::new(),
                value: String::new(),
            };
        };

        let (name, value) = match rest.split_once(" value ") {
            Some((n, v)) => (n.trim(), v.trim()),
            None => (rest.trim(), ""),
        };

        UciInput::SetOption {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    fn decode_go(&self, input: &str) -> UciInput {
        UciInput::Go(GoParams {
            infinite: input.contains("infinite"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_commands() {
        assert!(matches!(Decoder.decode("uci"), UciInput::Uci));
        assert!(matches!(Decoder.decode("isready"), UciInput::IsReady));
        assert!(matches!(Decoder.decode("ucinewgame"), UciInput::UciNewGame));
        assert!(matches!(Decoder.decode("stop"), UciInput::Stop));
        assert!(matches!(Decoder.decode("quit"), UciInput::Quit));
    }

    #[test]
    fn test_debug() {
        assert!(matches!(Decoder.decode("debug on"), UciInput::Debug(true)));
        assert!(matches!(
            Decoder.decode("debug off"),
            UciInput::Debug(false)
        ));
    }

    #[test]
    fn test_setoption_with_value() {
        let UciInput::SetOption { name, value } = Decoder.decode("setoption name Hash value 256")
        else {
            panic!("Expected SetOption")
        };
        assert_eq!(name, "Hash");
        assert_eq!(value, "256");
    }

    #[test]
    fn test_setoption_without_value() {
        let UciInput::SetOption { name, value } = Decoder.decode("setoption name Clear Hash")
        else {
            panic!("Expected SetOption")
        };
        assert_eq!(name, "Clear Hash");
        assert_eq!(value, "");
    }

    #[test]
    fn test_setoption_malformed() {
        // Missing "name" keyword - returns SetOption with empty name for error handling
        let UciInput::SetOption { name, value } = Decoder.decode("setoption invalid") else {
            panic!("Expected SetOption")
        };
        assert_eq!(name, "");
        assert_eq!(value, "");

        // Also test other malformed variants
        let UciInput::SetOption { name, value } = Decoder.decode("setoption value 123") else {
            panic!("Expected SetOption")
        };
        assert_eq!(name, "");
        assert_eq!(value, "");
    }

    #[test]
    fn test_go_infinite() {
        let UciInput::Go(params) = Decoder.decode("go infinite") else {
            panic!("Expected Go")
        };
        assert!(params.infinite);
        assert!(params.wtime.is_none());
    }

    #[test]
    fn test_go_with_time() {
        let UciInput::Go(params) = Decoder.decode("go wtime 60000 btime 60000 winc 1000 binc 1000")
        else {
            panic!("Expected Go")
        };
        assert!(!params.infinite);
        assert_eq!(params.wtime, Some(60000));
        assert_eq!(params.btime, Some(60000));
        assert_eq!(params.winc, Some(1000));
        assert_eq!(params.binc, Some(1000));
    }

    #[test]
    fn test_go_depth() {
        let UciInput::Go(params) = Decoder.decode("go depth 20") else {
            panic!("Expected Go")
        };
        assert_eq!(params.depth, Some(20));
    }

    #[test]
    fn test_go_movetime() {
        let UciInput::Go(params) = Decoder.decode("go movetime 5000") else {
            panic!("Expected Go")
        };
        assert_eq!(params.move_time, Some(5000));
    }

    #[test]
    fn test_position_startpos() {
        let UciInput::Position {
            board,
            game_history,
        } = Decoder.decode("position startpos")
        else {
            panic!("Expected Position")
        };
        assert_eq!(board, Board::default());
        assert!(game_history.is_empty());
    }

    #[test]
    fn test_position_startpos_with_moves() {
        let UciInput::Position {
            board,
            game_history,
        } = Decoder.decode("position startpos moves e2e4 e7e5")
        else {
            panic!("Expected Position")
        };
        assert_ne!(board, Board::default());
        assert_eq!(game_history.len(), 2);
    }

    #[test]
    fn test_position_fen() {
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let UciInput::Position {
            board,
            game_history,
        } = Decoder.decode(&format!("position fen {}", fen))
        else {
            panic!("Expected Position")
        };
        assert_eq!(board, Board::from_str(fen).unwrap());
        assert!(game_history.is_empty());
    }

    #[test]
    fn test_unknown_command() {
        assert!(matches!(
            Decoder.decode("invalid command"),
            UciInput::Unknown(_)
        ));
    }
}
