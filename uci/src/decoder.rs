use std::str::FromStr;

use ahash::AHashSet;
use cozy_chess::{Board, util::parse_uci_move};

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

            // Non-standard commands
            "d" => UciInput::Display,
            "bench" => UciInput::Bench,

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
            Board::default()
        };

        // Track positions seen in the game (not including the current position).
        // The current position will be the search root (included in search stack)
        let mut game_history = AHashSet::new();

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
        // Button-type options have no value.
        //
        // TODO: return Result or add an InvalidCommand variant instead of
        // signalling malformed input via an empty name.
        let Some(rest) = input.strip_prefix("setoption name ") else {
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
            nodes: extract_numeric_param(input, "nodes"),
            soft_nodes: extract_numeric_param(input, "softnodes"),
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

    fn decode(input: &str) -> UciInput {
        Decoder.decode(input)
    }

    #[test]
    fn keyword_commands() {
        assert!(matches!(decode("uci"), UciInput::Uci));
        assert!(matches!(decode("isready"), UciInput::IsReady));
        assert!(matches!(decode("ucinewgame"), UciInput::UciNewGame));
        assert!(matches!(decode("stop"), UciInput::Stop));
        assert!(matches!(decode("quit"), UciInput::Quit));
        assert!(matches!(decode("debug on"), UciInput::Debug(true)));
        assert!(matches!(decode("debug off"), UciInput::Debug(false)));
        assert!(matches!(decode("invalid command"), UciInput::Unknown(_)));
    }

    #[test]
    fn setoption() {
        let cases: &[(&str, &str, &str)] = &[
            ("setoption name Hash value 256", "Hash", "256"),
            ("setoption name Clear Hash", "Clear Hash", ""),
            // Malformed inputs surface as empty name for error handling upstream.
            ("setoption invalid", "", ""),
            ("setoption value 123", "", ""),
        ];
        for (input, want_name, want_value) in cases {
            let UciInput::SetOption { name, value } = decode(input) else {
                panic!("expected SetOption for {input:?}");
            };
            assert_eq!((name.as_str(), value.as_str()), (*want_name, *want_value));
        }
    }

    fn go(input: &str) -> GoParams {
        match decode(input) {
            UciInput::Go(p) => p,
            _ => panic!("expected Go for {input:?}"),
        }
    }

    #[test]
    fn go_variants() {
        let infinite = go("go infinite");
        assert!(infinite.infinite);
        assert!(infinite.wtime.is_none());

        let timed = go("go wtime 60000 btime 60000 winc 1000 binc 1000");
        assert!(!timed.infinite);
        assert_eq!(timed.wtime, Some(60000));
        assert_eq!(timed.btime, Some(60000));
        assert_eq!(timed.winc, Some(1000));
        assert_eq!(timed.binc, Some(1000));

        assert_eq!(go("go depth 20").depth, Some(20));
        assert_eq!(go("go movetime 5000").move_time, Some(5000));

        let hard_nodes = go("go nodes 100000");
        assert_eq!(hard_nodes.nodes, Some(100000));
        assert!(hard_nodes.depth.is_none());
        assert!(hard_nodes.move_time.is_none());

        let soft_nodes = go("go softnodes 30000");
        assert_eq!(soft_nodes.soft_nodes, Some(30000));
        assert!(soft_nodes.nodes.is_none());
    }

    #[test]
    fn position_variants() {
        let UciInput::Position {
            board,
            game_history,
        } = decode("position startpos")
        else {
            panic!("expected Position");
        };
        assert_eq!(board, Board::default());
        assert!(game_history.is_empty());

        let UciInput::Position {
            board,
            game_history,
        } = decode("position startpos moves e2e4 e7e5")
        else {
            panic!("expected Position");
        };
        assert_ne!(board, Board::default());
        assert_eq!(game_history.len(), 2);

        let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let UciInput::Position {
            board,
            game_history,
        } = decode(&format!("position fen {fen}"))
        else {
            panic!("expected Position");
        };
        assert_eq!(board, Board::from_str(fen).unwrap());
        assert!(game_history.is_empty());
    }
}
