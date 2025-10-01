use chess::{
    get_bishop_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard, Board, Color,
    Piece, Square, EMPTY,
};
use evaluation::piece_values::PieceValues;

#[derive(Clone, Copy, Debug)]
pub struct ThreatMap {
    /// Threats against side-to-move's pieces
    pub my_threats: BitBoard,
    /// Threats against opponent's pieces
    pub their_threats: BitBoard,
}

impl ThreatMap {
    pub fn new(board: &Board, _phase: f32, _piece_values: &PieceValues) -> Self {
        let us = board.side_to_move();
        let them = !us;
        let occupied = board.combined();

        let our_pieces = board.color_combined(us);
        let their_pieces = board.color_combined(them);

        let our_pawns = board.pieces(Piece::Pawn) & our_pieces;
        let our_knights = board.pieces(Piece::Knight) & our_pieces;
        let our_bishops = board.pieces(Piece::Bishop) & our_pieces;
        let our_rooks = board.pieces(Piece::Rook) & our_pieces;
        let our_queens = board.pieces(Piece::Queen) & our_pieces;

        let their_pawns = board.pieces(Piece::Pawn) & their_pieces;
        let their_knights = board.pieces(Piece::Knight) & their_pieces;
        let their_bishops = board.pieces(Piece::Bishop) & their_pieces;
        let their_rooks = board.pieces(Piece::Rook) & their_pieces;
        let their_queens = board.pieces(Piece::Queen) & their_pieces;

        let our_minors = our_knights | our_bishops;
        let our_majors = our_rooks | our_queens;
        let our_non_pawns = our_minors | our_majors;

        let their_minors = their_knights | their_bishops;
        let their_majors = their_rooks | their_queens;
        let their_non_pawns = their_minors | their_majors;

        let my_threats = compute_threats(
            their_pawns,
            their_knights,
            their_bishops,
            their_rooks,
            them,
            our_non_pawns,
            our_majors,
            our_queens,
            *occupied,
        );

        let their_threats = compute_threats(
            our_pawns,
            our_knights,
            our_bishops,
            our_rooks,
            us,
            their_non_pawns,
            their_majors,
            their_queens,
            *occupied,
        );

        Self {
            my_threats,
            their_threats,
        }
    }

    #[inline]
    pub fn my_threat_count(&self) -> u32 {
        self.my_threats.popcnt()
    }

    #[inline]
    pub fn their_threat_count(&self) -> u32 {
        self.their_threats.popcnt()
    }

    #[inline]
    pub fn is_my_piece_threatened(&self, square: Square) -> bool {
        self.my_threats & BitBoard::from_square(square) != EMPTY
    }
}

/// Compute threats from attacker's pieces against defender's pieces
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn compute_threats(
    attacker_pawns: BitBoard,
    attacker_knights: BitBoard,
    attacker_bishops: BitBoard,
    attacker_rooks: BitBoard,
    attacker_color: Color,
    defender_non_pawns: BitBoard,
    defender_majors: BitBoard,
    defender_queens: BitBoard,
    occupied: BitBoard,
) -> BitBoard {
    let mut threats = EMPTY;

    // Pawn threats against non-pawn pieces
    if defender_non_pawns != EMPTY {
        threats |= pawn_attacks_all(attacker_pawns, attacker_color, defender_non_pawns);
    }

    // Minor piece threats against major pieces
    if defender_majors != EMPTY {
        for knight_sq in attacker_knights {
            threats |= get_knight_moves(knight_sq) & defender_majors;
        }
        for bishop_sq in attacker_bishops {
            threats |= get_bishop_moves(bishop_sq, occupied) & defender_majors;
        }
    }

    // Rook threats against queens
    if defender_queens != EMPTY {
        for rook_sq in attacker_rooks {
            threats |= get_rook_moves(rook_sq, occupied) & defender_queens;
        }
    }

    threats
}

#[inline(always)]
fn pawn_attacks_all(pawns: BitBoard, color: Color, targets: BitBoard) -> BitBoard {
    let mut attacks = EMPTY;
    for pawn_sq in pawns {
        attacks |= get_pawn_attacks(pawn_sq, color, targets);
    }
    attacks
}
