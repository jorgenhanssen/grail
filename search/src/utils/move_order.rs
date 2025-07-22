use chess::{BitBoard, Board, ChessMove, MoveGen, Piece};

#[inline(always)]
pub fn ordered_moves(
    board: &Board,
    mask: Option<BitBoard>,
    depth: u8,
    pv_move: &[ChessMove],
    tt_move: Option<ChessMove>,
    killer_moves: &[[Option<ChessMove>; 2]],
) -> Vec<ChessMove> {
    let mut legal = MoveGen::new_legal(board);
    if let Some(mask) = mask {
        legal.set_iterator_mask(mask);
    }

    let mut moves_with_priority: Vec<(ChessMove, i16)> = Vec::with_capacity(64); // Rough estimate; chess max ~218

    let killers = &killer_moves[depth as usize];
    let pv = pv_move.get(depth as usize).cloned();

    for mov in legal {
        let mut priority = move_priority(&mov, board);

        if Some(mov) == tt_move {
            priority = priority.max(MAX_PRIORITY + 1);
        }
        if Some(mov) == pv {
            priority = priority.max(MAX_PRIORITY + 2);
        }
        if killers.iter().any(|&k| k == Some(mov)) {
            priority = priority.max(CAPTURE_PRIORITY - 1);
        }

        moves_with_priority.push((mov, priority));
    }

    moves_with_priority.sort_unstable_by_key(|&(_, p)| -p);

    moves_with_priority.into_iter().map(|(m, _)| m).collect()
}

// (Your constants and move_priority function remain unchanged;
// they're already efficient. Here's a quick copy for completeness.)

// Piece moves get base priority (lowest)
pub const MIN_PRIORITY: i16 = 0;

pub const MIN_PIECE_PRIORITY: i16 = MIN_PRIORITY;
const PIECE_PRIORITY_KNIGHT: i16 = MIN_PIECE_PRIORITY + 1;
const PIECE_PRIORITY_BISHOP: i16 = MIN_PIECE_PRIORITY + 2;
const PIECE_PRIORITY_ROOK: i16 = MIN_PIECE_PRIORITY + 3;
const PIECE_PRIORITY_QUEEN: i16 = MIN_PIECE_PRIORITY + 4;
pub const MAX_PIECE_PRIORITY: i16 = PIECE_PRIORITY_QUEEN;

// Captures get medium priority (MVV-LVA values 10-55)
pub const MIN_CAPTURE_PRIORITY: i16 = MIN_PRIORITY + 100;
pub const CAPTURE_PRIORITY: i16 = MIN_CAPTURE_PRIORITY;
pub const MAX_CAPTURE_PRIORITY: i16 = MIN_CAPTURE_PRIORITY + 55;

// Promotions get highest priority
pub const MIN_PROMOTION_PRIORITY: i16 = MIN_PRIORITY + 200;
const PROMOTION_PRIORITY_KNIGHT: i16 = MIN_PROMOTION_PRIORITY + 1;
const PROMOTION_PRIORITY_BISHOP: i16 = MIN_PROMOTION_PRIORITY + 2;
const PROMOTION_PRIORITY_ROOK: i16 = MIN_PROMOTION_PRIORITY + 3;
const PROMOTION_PRIORITY_QUEEN: i16 = MIN_PROMOTION_PRIORITY + 4;
pub const MAX_PROMOTION_PRIORITY: i16 = PROMOTION_PRIORITY_QUEEN;

pub const MAX_PRIORITY: i16 = MAX_PROMOTION_PRIORITY;

// MVV-LVA table
// king, queen, rook, bishop, knight, pawn
const MVV_LVA: [[i16; 6]; 6] = [
    [0, 0, 0, 0, 0, 0],       // victim King
    [50, 51, 52, 53, 54, 55], // victim Queen
    [40, 41, 42, 43, 44, 45], // victim Rook
    [30, 31, 32, 33, 34, 35], // victim Bishop
    [20, 21, 22, 23, 24, 25], // victim Knight
    [10, 11, 12, 13, 14, 15], // victim Pawn
];

// Helper function to convert Piece to array index
#[inline]
fn mvva_lva_index(piece: Piece) -> usize {
    match piece {
        Piece::King => 0,
        Piece::Queen => 1,
        Piece::Rook => 2,
        Piece::Bishop => 3,
        Piece::Knight => 4,
        Piece::Pawn => 5,
    }
}

#[inline(always)]
fn move_priority(mov: &ChessMove, board: &Board) -> i16 {
    // Check for promotions first
    if let Some(promotion) = mov.get_promotion() {
        return match promotion {
            Piece::Queen => PROMOTION_PRIORITY_QUEEN,
            Piece::Rook => PROMOTION_PRIORITY_ROOK,
            Piece::Bishop => PROMOTION_PRIORITY_BISHOP,
            Piece::Knight => PROMOTION_PRIORITY_KNIGHT,
            _ => 0,
        };
    }

    let attacker = board.piece_on(mov.get_source()).unwrap();
    if let Some(victim) = board.piece_on(mov.get_dest()) {
        return CAPTURE_PRIORITY + MVV_LVA[mvva_lva_index(victim)][mvva_lva_index(attacker)];
    }

    // Nudge move ordering to prefer more valuable pieces
    match attacker {
        Piece::Queen => PIECE_PRIORITY_QUEEN,
        Piece::Rook => PIECE_PRIORITY_ROOK,
        Piece::Bishop => PIECE_PRIORITY_BISHOP,
        Piece::Knight => PIECE_PRIORITY_KNIGHT,
        _ => MIN_PRIORITY,
    }
}
