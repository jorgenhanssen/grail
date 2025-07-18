use std::fmt;

use chess::{Board, ChessMove, File, GameResult, MoveGen, Piece, Rank, Square};

const STANDARD_POSITION_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug)]
pub struct GameOutcome {
    pub starting_position: String,
    pub white_name: String,
    pub black_name: String,
    pub result: GameResult,
    pub moves: Vec<ChessMove>,
    pub positions: Vec<Board>, // for SAN conversion
}

impl GameOutcome {
    pub fn to_pgn(&self) -> String {
        let mut pgn = String::with_capacity(512);

        pgn.push_str(&format!("[White \"{}\"]\n", self.white_name));
        pgn.push_str(&format!("[Black \"{}\"]\n", self.black_name));

        let result_str = game_result_to_pgn(self.result);
        pgn.push_str(&format!("[Result \"{}\"]\n", result_str));

        // Add variant and FEN headers if not starting from standard position
        let is_standard_position = self.starting_position == STANDARD_POSITION_FEN;

        if !is_standard_position {
            pgn.push_str("[Variant \"From Position\"]\n");
            pgn.push_str(&format!("[FEN \"{}\"]\n", &self.starting_position));
        }

        pgn.push('\n');

        // Extract fullmove number from original FEN (last component, safer than index 5)
        let fen_parts: Vec<&str> = self.starting_position.split_whitespace().collect();
        let starting_move_number: u32 = fen_parts.last().and_then(|s| s.parse().ok()).unwrap_or(1);

        // Check if the starting position has white or black to move (from original FEN part 1)
        let starting_color_str = fen_parts.get(1).unwrap_or(&"w");
        let white_started = *starting_color_str == "w";

        // Build movetext separately to allow trimming trailing space
        let mut movetext = String::with_capacity(256);

        for (i, mv) in self.moves.iter().enumerate() {
            let board = &self.positions[i];
            let san_move = to_san(board, *mv);

            // In PGN if black started, the first move skips white and is noted as N... MOVE
            if i == 0 && !white_started {
                movetext.push_str(&format!("{}... {} ", starting_move_number, san_move));
                continue;
            }

            let move_number = if white_started {
                starting_move_number + (i / 2) as u32
            } else {
                starting_move_number + ((i + 1) / 2) as u32
            };

            // Determine if this is the start of a pair (white's move, needs "N. " prefix)
            let is_prefix_move = if white_started {
                i % 2 == 0
            } else {
                i % 2 != 0
            };

            if is_prefix_move {
                movetext.push_str(&format!("{}. {} ", move_number, san_move));
            } else {
                movetext.push_str(&format!("{} ", san_move));
            }
        }

        let movetext = movetext.trim_end();
        pgn.push_str(movetext);
        if !movetext.is_empty() {
            pgn.push(' ');
        }
        pgn.push_str(result_str);

        pgn
    }
}

impl fmt::Display for GameOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} vs {} - {} moves: {}",
            self.white_name,
            self.black_name,
            self.moves.len(),
            game_result_to_pgn(self.result)
        )
    }
}

// Backwards code from Chess crate which only has from_san:

fn to_san(board: &Board, mv: ChessMove) -> String {
    let from_square = mv.get_source();
    let to_square = mv.get_dest();
    let moving_piece = board.piece_on(from_square).unwrap();
    let is_capture = board.piece_on(to_square).is_some()
        || (moving_piece == Piece::Pawn && from_square.get_file() != to_square.get_file());

    // Handle castling
    if moving_piece == Piece::King {
        let king_file = from_square.get_file();
        let dest_file = to_square.get_file();

        if king_file == File::E {
            if dest_file == File::G {
                return "O-O".to_string();
            } else if dest_file == File::C {
                return "O-O-O".to_string();
            }
        }
    }

    let mut san = String::new();

    // Add piece letter (except for pawns)
    if moving_piece != Piece::Pawn {
        san.push(piece_to_char(moving_piece));

        // Check for disambiguation
        let disambiguation = get_disambiguation(board, mv, moving_piece);
        san.push_str(&disambiguation);
    } else if is_capture {
        // For pawn captures, include source file
        san.push(file_to_char(from_square.get_file()));
    }

    // Add capture notation
    if is_capture {
        san.push('x');
    }

    // Add destination square
    san.push_str(&square_to_string(to_square));

    // Add promotion
    if let Some(promotion) = mv.get_promotion() {
        san.push('=');
        san.push(piece_to_char(promotion));
    }

    // TODO: Add check/checkmate notation (+/#)
    // This would require making the move and checking the resulting position

    san
}

#[inline]

fn piece_to_char(piece: Piece) -> char {
    match piece {
        Piece::Pawn => 'P',
        Piece::Knight => 'N',
        Piece::Bishop => 'B',
        Piece::Rook => 'R',
        Piece::Queen => 'Q',
        Piece::King => 'K',
    }
}

#[inline]
fn file_to_char(file: File) -> char {
    match file {
        File::A => 'a',
        File::B => 'b',
        File::C => 'c',
        File::D => 'd',
        File::E => 'e',
        File::F => 'f',
        File::G => 'g',
        File::H => 'h',
    }
}

#[inline]
fn rank_to_char(rank: Rank) -> char {
    match rank {
        Rank::First => '1',
        Rank::Second => '2',
        Rank::Third => '3',
        Rank::Fourth => '4',
        Rank::Fifth => '5',
        Rank::Sixth => '6',
        Rank::Seventh => '7',
        Rank::Eighth => '8',
    }
}

#[inline]
fn square_to_string(square: Square) -> String {
    format!(
        "{}{}",
        file_to_char(square.get_file()),
        rank_to_char(square.get_rank())
    )
}

fn get_disambiguation(board: &Board, mv: ChessMove, piece: Piece) -> String {
    let dest = mv.get_dest();
    let source = mv.get_source();

    // Find all other pieces of the same type that could move to the same destination
    let mut same_piece_moves = Vec::new();
    for legal_move in MoveGen::new_legal(board) {
        if legal_move.get_dest() == dest
            && legal_move.get_source() != source
            && board.piece_on(legal_move.get_source()) == Some(piece)
        {
            same_piece_moves.push(legal_move);
        }
    }

    if same_piece_moves.is_empty() {
        return String::new();
    }

    // Check if file disambiguation is sufficient
    let source_file = source.get_file();
    let file_conflicts = same_piece_moves
        .iter()
        .any(|mv| mv.get_source().get_file() == source_file);

    if !file_conflicts {
        return file_to_char(source_file).to_string();
    }

    // Check if rank disambiguation is sufficient
    let source_rank = source.get_rank();
    let rank_conflicts = same_piece_moves
        .iter()
        .any(|mv| mv.get_source().get_rank() == source_rank);

    if !rank_conflicts {
        return rank_to_char(source_rank).to_string();
    }

    // Use full square disambiguation
    square_to_string(source)
}

#[inline]
fn game_result_to_pgn(result: GameResult) -> &'static str {
    match result {
        GameResult::WhiteCheckmates => "1-0",
        GameResult::BlackCheckmates => "0-1",
        GameResult::WhiteResigns => "0-1",
        GameResult::BlackResigns => "1-0",
        GameResult::Stalemate => "1/2-1/2",
        GameResult::DrawAccepted => "1/2-1/2",
        GameResult::DrawDeclared => "1/2-1/2",
    }
}
