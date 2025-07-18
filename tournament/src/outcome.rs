use std::fmt;

use chess::{Board, ChessMove, Color, File, GameResult, MoveGen, Piece, Rank, Square};

use crate::utils::index_to_color;

#[derive(Debug)]
pub struct GameOutcome {
    pub white_name: String,
    pub black_name: String,
    pub result: GameResult,
    pub moves: Vec<ChessMove>,
    pub positions: Vec<Board>, // for SAN conversion
}

impl GameOutcome {
    pub fn to_pgn(&self) -> String {
        let mut pgn = String::new();

        pgn.push_str(&format!("[White \"{}\"]\n", self.white_name));
        pgn.push_str(&format!("[Black \"{}\"]\n", self.black_name));

        let result_str = game_result_to_pgn(self.result);
        pgn.push_str(&format!("[Result \"{}\"]\n", result_str));

        pgn.push_str("\n");

        for (i, mv) in self.moves.iter().enumerate() {
            let board = &self.positions[i];
            let san_move = to_san(board, *mv);

            match index_to_color(i % 2) {
                Color::White => pgn.push_str(&format!("{}. {} ", (i / 2) + 1, san_move)),
                Color::Black => pgn.push_str(&format!("{} ", san_move)),
            }
        }

        pgn.push_str(&format!("{}", result_str));

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
