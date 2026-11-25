use cozy_chess::{
    BitBoard, Board, Color, Piece, get_bishop_moves, get_king_moves, get_knight_moves,
    get_pawn_attacks, get_rook_moves,
};

use crate::attacks::get_queen_moves;
use crate::threats::threat_targets;

/// Per-color attack/threat/support bitboards, computed once and reused
/// throughout evaluation.
#[derive(Clone, Copy, Debug)]
pub struct BoardMetrics {
    pub attacks: [BitBoard; Color::NUM],

    /// Pieces under attack by lesser enemy pieces.
    pub threats: [BitBoard; Color::NUM],

    /// Own pieces defended by own pieces.
    pub support: [BitBoard; Color::NUM],
}

impl BoardMetrics {
    /// Compute all board metrics in a single pass.
    pub fn new(board: &Board) -> Self {
        let (white_attacks, black_threats) = compute(board, Color::White);
        let (black_attacks, white_threats) = compute(board, Color::Black);

        let white_support = white_attacks & board.colors(Color::White);
        let black_support = black_attacks & board.colors(Color::Black);

        Self {
            attacks: [white_attacks, black_attacks],
            threats: [white_threats, black_threats],
            support: [white_support, black_support],
        }
    }
}

/// Compute attacks and threats for one color in a single pass.
fn compute(board: &Board, color: Color) -> (BitBoard, BitBoard) {
    let occupied = board.occupied();
    let mut attacks = BitBoard::EMPTY;
    let mut threats = BitBoard::EMPTY;

    // TODO: Consider refactoring these into a helper function
    let targets = threat_targets(board, !color, Piece::Pawn);
    for sq in board.colored_pieces(color, Piece::Pawn) {
        let squares = get_pawn_attacks(sq, color);
        attacks |= squares;
        threats |= squares & targets;
    }

    let targets = threat_targets(board, !color, Piece::Knight);
    for sq in board.colored_pieces(color, Piece::Knight) {
        let squares = get_knight_moves(sq);
        attacks |= squares;
        threats |= squares & targets;
    }

    let targets = threat_targets(board, !color, Piece::Bishop);
    for sq in board.colored_pieces(color, Piece::Bishop) {
        let squares = get_bishop_moves(sq, occupied);
        attacks |= squares;
        threats |= squares & targets;
    }

    let targets = threat_targets(board, !color, Piece::Rook);
    for sq in board.colored_pieces(color, Piece::Rook) {
        let squares = get_rook_moves(sq, occupied);
        attacks |= squares;
        threats |= squares & targets;
    }

    let targets = threat_targets(board, !color, Piece::Queen);
    for sq in board.colored_pieces(color, Piece::Queen) {
        let squares = get_queen_moves(sq, occupied);
        attacks |= squares;
        threats |= squares & targets;
    }

    let targets = threat_targets(board, !color, Piece::King);
    for sq in board.colored_pieces(color, Piece::King) {
        let squares = get_king_moves(sq);
        attacks |= squares;
        threats |= squares & targets;
    }

    (attacks, threats)
}
