use cozy_chess::{
    BitBoard, Board, Color, Piece, get_bishop_moves, get_king_moves, get_knight_moves,
    get_pawn_attacks, get_rook_moves,
};

use crate::attacks::get_queen_moves;

/// Precomputed board metrics for evaluation.
///
/// These metrics are expensive to compute but reused multiple times
/// during position evaluation.
#[derive(Clone, Copy, Debug)]
pub struct BoardMetrics {
    /// Attack bitboards for each color (all squares attacked by that color).
    pub attacks: [BitBoard; Color::NUM],

    /// Threats bitboards: opponent's valuable pieces (non-pawns) under attack.
    /// `threats[White]` = White's pieces threatened by Black.
    pub threats: [BitBoard; Color::NUM],

    /// Support bitboards: own pieces defended by own pieces.
    /// `support[White]` = White pieces defended by White.
    pub support: [BitBoard; Color::NUM],
}

impl BoardMetrics {
    /// Compute all board metrics in a single pass.
    pub fn new(board: &Board) -> Self {
        let all_pieces = board.occupied();
        let white_pieces = board.colors(Color::White);
        let black_pieces = board.colors(Color::Black);

        let pawns = board.pieces(Piece::Pawn);
        let knights = board.pieces(Piece::Knight);
        let bishops = board.pieces(Piece::Bishop);
        let rooks = board.pieces(Piece::Rook);
        let queens = board.pieces(Piece::Queen);

        let white_pawns = pawns & white_pieces;
        let black_pawns = pawns & black_pieces;
        let white_knights = knights & white_pieces;
        let black_knights = knights & black_pieces;
        let white_bishops = bishops & white_pieces;
        let black_bishops = bishops & black_pieces;
        let white_rooks = rooks & white_pieces;
        let black_rooks = rooks & black_pieces;
        let white_queens = queens & white_pieces;
        let black_queens = queens & black_pieces;

        // Compute piece groupings for threat detection
        let white_minors = white_knights | white_bishops;
        let black_minors = black_knights | black_bishops;
        let white_majors = white_rooks | white_queens;
        let black_majors = black_rooks | black_queens;
        let white_non_pawns = white_minors | white_majors;
        let black_non_pawns = black_minors | black_majors;

        let white_king = board.king(Color::White);
        let black_king = board.king(Color::Black);

        let (white_attacks, black_threats) = compute(
            Color::White,
            white_pawns,
            white_knights,
            white_bishops,
            white_rooks,
            white_queens,
            white_king,
            black_non_pawns,
            black_majors,
            black_queens,
            all_pieces,
        );

        let (black_attacks, white_threats) = compute(
            Color::Black,
            black_pawns,
            black_knights,
            black_bishops,
            black_rooks,
            black_queens,
            black_king,
            white_non_pawns,
            white_majors,
            white_queens,
            all_pieces,
        );

        // Which of our pieces are defended by our own pieces
        let white_support = white_attacks & white_pieces;
        let black_support = black_attacks & black_pieces;

        Self {
            attacks: [white_attacks, black_attacks],
            threats: [white_threats, black_threats],
            support: [white_support, black_support],
        }
    }
}

/// Compute attacks and threats for one color in a single pass.
#[allow(clippy::too_many_arguments)]
fn compute(
    color: Color,
    pawns: BitBoard,
    knights: BitBoard,
    bishops: BitBoard,
    rooks: BitBoard,
    queens: BitBoard,
    king: cozy_chess::Square,
    opponent_non_pawns: BitBoard,
    opponent_majors: BitBoard,
    opponent_queens: BitBoard,
    all_pieces: BitBoard,
) -> (BitBoard, BitBoard) {
    let mut attacks = BitBoard::EMPTY;
    let mut threats = BitBoard::EMPTY;

    // Pawns: threaten any non-pawn piece
    for sq in pawns {
        let squares = get_pawn_attacks(sq, color);
        attacks |= squares;
        threats |= squares & opponent_non_pawns;
    }

    // Knights: threaten major pieces (rooks, queens)
    for sq in knights {
        let squares = get_knight_moves(sq);
        attacks |= squares;
        threats |= squares & opponent_majors;
    }

    // Bishops: threaten major pieces (rooks, queens)
    for sq in bishops {
        let squares = get_bishop_moves(sq, all_pieces);
        attacks |= squares;
        threats |= squares & opponent_majors;
    }

    // Rooks: threaten queens
    for sq in rooks {
        let squares = get_rook_moves(sq, all_pieces);
        attacks |= squares;
        threats |= squares & opponent_queens;
    }

    // Queens: don't generate threats (nothing more valuable to threaten)
    for sq in queens {
        let squares = get_queen_moves(sq, all_pieces);
        attacks |= squares;
    }

    // Kings: Also don't generate threats (can be discussed, I suppose)
    attacks |= get_king_moves(king);

    (attacks, threats)
}
