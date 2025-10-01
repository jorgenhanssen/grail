use chess::{
    get_bishop_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard, Board, Color,
    Piece, Square, EMPTY,
};

#[derive(Clone, Copy, Debug)]
pub struct ThreatMap {
    // Threats against side-to-move's pieces
    pub threats: BitBoard,
}

impl ThreatMap {
    pub fn new(board: &Board) -> Self {
        let me = board.side_to_move();
        let them = !me;
        let occupied = board.combined();

        let my_pieces = board.color_combined(me);
        let their_pieces = board.color_combined(them);

        let my_knights = board.pieces(Piece::Knight) & my_pieces;
        let my_bishops = board.pieces(Piece::Bishop) & my_pieces;
        let my_rooks = board.pieces(Piece::Rook) & my_pieces;
        let my_queens = board.pieces(Piece::Queen) & my_pieces;

        let their_pawns = board.pieces(Piece::Pawn) & their_pieces;
        let their_knights = board.pieces(Piece::Knight) & their_pieces;
        let their_bishops = board.pieces(Piece::Bishop) & their_pieces;
        let their_rooks = board.pieces(Piece::Rook) & their_pieces;

        let my_minors = my_knights | my_bishops;
        let my_majors = my_rooks | my_queens;
        let my_non_pawns = my_minors | my_majors;

        let threats = compute_threats(
            their_pawns,
            their_knights,
            their_bishops,
            their_rooks,
            them,
            my_non_pawns,
            my_majors,
            my_queens,
            *occupied,
        );

        Self { threats }
    }

    #[inline(always)]
    pub fn is_threatened(&self, square: Square) -> bool {
        self.threats & BitBoard::from_square(square) != EMPTY
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
