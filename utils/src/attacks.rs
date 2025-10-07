use chess::{
    get_bishop_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard, Board, Color,
    Piece, NUM_COLORS,
};

#[derive(Clone, Copy, Debug)]
pub struct Attacks {
    // Total space (number of squares attacked/controlled) for each color
    pub space: [i16; NUM_COLORS],

    // Attack bitboards for each color (all squares attacked by that color)
    pub attacks: [BitBoard; NUM_COLORS],

    // Threats: which valuable pieces (non-pawns) are attacked by opponent
    pub threats: [BitBoard; NUM_COLORS],
}

impl Attacks {
    pub fn new(board: &Board) -> Self {
        let all_pieces = board.combined();
        let white_pieces = board.color_combined(Color::White);
        let black_pieces = board.color_combined(Color::Black);

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

        let (white_space, white_attacks) = compute_attacks_for_color(
            Color::White,
            *white_pieces,
            white_pawns,
            white_knights,
            white_bishops,
            white_rooks,
            white_queens,
            *all_pieces,
        );

        let (black_space, black_attacks) = compute_attacks_for_color(
            Color::Black,
            *black_pieces,
            black_pawns,
            black_knights,
            black_bishops,
            black_rooks,
            black_queens,
            *all_pieces,
        );

        // Pre-compute threats for both colors
        // Threats = opponent attacks & my valuable pieces (non-pawns)
        let white_non_pawns = *white_pieces & !pawns;
        let black_non_pawns = *black_pieces & !pawns;

        let white_threats = black_attacks & white_non_pawns;
        let black_threats = white_attacks & black_non_pawns;

        Self {
            space: [white_space, black_space],
            attacks: [white_attacks, black_attacks],
            threats: [white_threats, black_threats],
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_attacks_for_color(
    color: Color,
    my_pieces: BitBoard,
    pawns: BitBoard,
    knights: BitBoard,
    bishops: BitBoard,
    rooks: BitBoard,
    queens: BitBoard,
    all_pieces: BitBoard,
) -> (i16, BitBoard) {
    let mut space = 0i16;
    let mut attacks = BitBoard::default();

    // Pawn attacks
    for sq in pawns {
        let squares = get_pawn_attacks(sq, color, all_pieces);
        space += squares.popcnt() as i16;
        attacks |= squares;
    }

    // Knight attacks/space
    for sq in knights {
        let squares = get_knight_moves(sq);
        space += (squares & !my_pieces).popcnt() as i16;
        attacks |= squares;
    }

    // Bishop attacks/space
    for sq in bishops {
        let squares = get_bishop_moves(sq, all_pieces);
        space += (squares & !my_pieces).popcnt() as i16;
        attacks |= squares;
    }

    // Rook attacks/space
    for sq in rooks {
        let squares = get_rook_moves(sq, all_pieces);
        space += (squares & !my_pieces).popcnt() as i16;
        attacks |= squares;
    }

    // Queen attacks/space
    for sq in queens {
        let squares = get_bishop_moves(sq, all_pieces) | get_rook_moves(sq, all_pieces);
        space += (squares & !my_pieces).popcnt() as i16;
        attacks |= squares;
    }

    (space, attacks)
}
