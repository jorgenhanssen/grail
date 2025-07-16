use chess::{BitBoard, Board, ChessMove, MoveGen, Piece};

pub fn ordered_moves(
    board: &Board,
    preferred: Option<&[(ChessMove, i16)]>,
    mask: Option<BitBoard>,
) -> Vec<(ChessMove, i16)> {
    let mut legal = MoveGen::new_legal(board);
    if let Some(mask) = mask {
        legal.set_iterator_mask(mask);
    }

    let cap = legal.len();

    // Separate buckets: high = priority > 0, low = priority == 0
    let mut high = Vec::with_capacity(cap);
    let mut low = Vec::with_capacity(cap);

    for m in legal {
        // Try to find the move in the preferred list
        let mut priority = 0;
        if let Some(preferred) = preferred {
            for &(pm, s) in preferred {
                if pm == m {
                    priority = s;
                    break;
                }
            }
        }
        if priority == 0 {
            priority = move_priority(m, board);
        }

        // bucket by priority
        if priority > MIN_PRIORITY {
            high.push((m, priority));
        } else {
            low.push((m, priority));
        }
    }

    // Only sort high priority moves
    high.sort_unstable_by_key(|&(_, s)| -s);

    // Append low priority moves last
    high.extend(low);
    high
}

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
fn move_priority(mov: ChessMove, board: &Board) -> i16 {
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
