use cozy_chess::{
    get_bishop_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard, Board, Color,
    Piece,
};

#[derive(Clone, Copy, Debug)]
pub struct BoardMetrics {
    // Total space (number of squares attacked/controlled) for each color
    pub space: [i16; Color::NUM],

    // Attack bitboards for each color (all squares attacked by that color)
    pub attacks: [BitBoard; Color::NUM],

    // Threats: which valuable pieces (non-pawns) are attacked by opponent
    pub threats: [BitBoard; Color::NUM],

    // Support: which of our pieces are defended by our own pieces
    pub support: [BitBoard; Color::NUM],
}

impl BoardMetrics {
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

        let (white_space, white_attacks, black_threats) = compute(
            Color::White,
            white_pieces,
            white_pawns,
            white_knights,
            white_bishops,
            white_rooks,
            white_queens,
            black_non_pawns,
            black_majors,
            black_queens,
            all_pieces,
        );

        let (black_space, black_attacks, white_threats) = compute(
            Color::Black,
            black_pieces,
            black_pawns,
            black_knights,
            black_bishops,
            black_rooks,
            black_queens,
            white_non_pawns,
            white_majors,
            white_queens,
            all_pieces,
        );

        // Which of our pieces are defended by our own pieces
        let white_support = white_attacks & white_pieces;
        let black_support = black_attacks & black_pieces;

        Self {
            space: [white_space, black_space],
            attacks: [white_attacks, black_attacks],
            threats: [white_threats, black_threats],
            support: [white_support, black_support],
        }
    }
}

/// Compute attacks, space, and threats in a single pass
/// This is optimized to avoid iterating through pieces multiple times
#[allow(clippy::too_many_arguments)]
fn compute(
    color: Color,
    my_pieces: BitBoard,
    pawns: BitBoard,
    knights: BitBoard,
    bishops: BitBoard,
    rooks: BitBoard,
    queens: BitBoard,
    opponent_non_pawns: BitBoard,
    opponent_majors: BitBoard,
    opponent_queens: BitBoard,
    all_pieces: BitBoard,
) -> (i16, BitBoard, BitBoard) {
    let mut space = 0i16;
    let mut attacks = BitBoard::EMPTY;
    let mut threats = BitBoard::EMPTY;

    // Check if we need to compute threats at all
    let has_non_pawns = !opponent_non_pawns.is_empty();
    let has_majors = !opponent_majors.is_empty();
    let has_queens = !opponent_queens.is_empty();

    if !pawns.is_empty() {
        for sq in pawns {
            let squares = get_pawn_attacks(sq, color) & all_pieces;
            space += (squares & !my_pieces).len() as i16;
            attacks |= squares;
            // Pawns threaten any non-pawn piece
            if has_non_pawns {
                threats |= squares & opponent_non_pawns;
            }
        }
    }

    if !knights.is_empty() {
        for sq in knights {
            let squares = get_knight_moves(sq);
            space += (squares & !my_pieces).len() as i16;
            attacks |= squares;
            // Knights (minor pieces) threaten major pieces
            if has_majors {
                threats |= squares & opponent_majors;
            }
        }
    }

    if !bishops.is_empty() {
        for sq in bishops {
            let squares = get_bishop_moves(sq, all_pieces);
            space += (squares & !my_pieces).len() as i16;
            attacks |= squares;
            // Bishops (minor pieces) threaten major pieces
            if has_majors {
                threats |= squares & opponent_majors;
            }
        }
    }

    if !rooks.is_empty() {
        for sq in rooks {
            let squares = get_rook_moves(sq, all_pieces);
            space += (squares & !my_pieces).len() as i16;
            attacks |= squares;

            if has_queens {
                threats |= squares & opponent_queens;
            }
        }
    }

    if !queens.is_empty() {
        for sq in queens {
            let squares = get_bishop_moves(sq, all_pieces) | get_rook_moves(sq, all_pieces);
            space += (squares & !my_pieces).len() as i16;
            attacks |= squares;
        }
    }

    (space, attacks, threats)
}
