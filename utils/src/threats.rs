use cozy_chess::{BitBoard, Board, Color, Piece};

/// Pieces that would be threatened by the attacker piece
pub fn threat_targets(board: &Board, color: Color, attacker: Piece) -> BitBoard {
    let mut targets = BitBoard::EMPTY;
    for target in Piece::ALL {
        if is_threat(attacker, target) {
            targets |= board.pieces(target);
        }
    }
    targets & board.colors(color)
}

/// Candidate pieces that could threaten the target piece.
pub fn threat_attackers(board: &Board, color: Color, target: Piece) -> BitBoard {
    let mut attackers = BitBoard::EMPTY;
    for attacker in Piece::ALL {
        if is_threat(attacker, target) {
            attackers |= board.pieces(attacker);
        }
    }
    attackers & board.colors(color)
}

fn is_threat(attacker: Piece, target: Piece) -> bool {
    match attacker {
        // Pawns: threaten any non-pawn piece
        Piece::Pawn => !matches!(target, Piece::Pawn | Piece::King),
        // Knights/Bishop: threaten major pieces (rooks, queens)
        Piece::Knight | Piece::Bishop => matches!(target, Piece::Rook | Piece::Queen),
        // Rooks: threaten queens
        Piece::Rook => target == Piece::Queen,
        // Queens/King: don't generate threats (nothing more valuable to threaten)
        Piece::Queen | Piece::King => false,
    }
}
