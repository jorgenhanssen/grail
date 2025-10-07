use chess::{
    get_bishop_moves, get_knight_moves, get_pawn_attacks, get_rook_moves, BitBoard, Board, Color,
    Piece, NUM_COLORS,
};
use std::cell::OnceCell;

/// Position wrapper that lazily computes and caches attack map
/// This allows sharing expensive attack computations between:
/// - Threat detection (move ordering)
/// - Space evaluation (HCE)
pub struct Position<'a> {
    pub board: &'a Board,
    // Lazy-computed attack map - computed once and reused
    attack_map: OnceCell<AttackMap>,
}

impl<'a> Position<'a> {
    #[inline(always)]
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            attack_map: OnceCell::new(),
        }
    }

    /// Get or compute the attack map (computed once, cached for reuse)
    #[inline(always)]
    pub fn attack_map(&self) -> &AttackMap {
        self.attack_map.get_or_init(|| AttackMap::new(self.board))
    }

    /// Get threats against side-to-move's pieces (cached)
    #[inline(always)]
    pub fn threats(&self) -> BitBoard {
        let me = self.board.side_to_move();
        self.attack_map().threats_for(me)
    }

    /// Get threats against specific color's pieces (cached)
    #[inline(always)]
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.attack_map().threats_for(color)
    }
}

/// Pre-computed attack patterns for all pieces
/// This is computed once and reused for both threats and space
#[derive(Clone, Copy, Debug)]
pub struct AttackMap {
    // Total space (number of squares attacked/controlled) for each color
    space: [i16; NUM_COLORS],

    // Attack bitboards for each color (all squares attacked by that color)
    attacks: [BitBoard; NUM_COLORS],

    // Threats: which valuable pieces (non-pawns) are attacked by opponent
    // threats[color] = opponent's attacks & my non-pawns
    threats: [BitBoard; NUM_COLORS],
}

impl AttackMap {
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

        let (white_space, white_attacks) = Self::compute_attacks_for_color(
            Color::White,
            *white_pieces,
            white_pawns,
            white_knights,
            white_bishops,
            white_rooks,
            white_queens,
            *all_pieces,
        );

        let (black_space, black_attacks) = Self::compute_attacks_for_color(
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

    #[inline(always)]
    pub fn space_for(&self, color: Color) -> i16 {
        self.space[color.to_index()]
    }

    #[inline(always)]
    pub fn attacks_for(&self, color: Color) -> BitBoard {
        self.attacks[color.to_index()]
    }

    #[inline(always)]
    pub fn threats_for(&self, color: Color) -> BitBoard {
        self.threats[color.to_index()]
    }
}
