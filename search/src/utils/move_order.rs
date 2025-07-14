use chess::{BitBoard, Board, ChessMove, MoveGen, Piece};

pub fn get_ordered_moves(
    board: &Board,
    preferred: Option<&[(ChessMove, i32)]>,
    mask: Option<BitBoard>,
) -> Vec<(ChessMove, i32)> {
    let mut legal = MoveGen::new_legal(board);
    if let Some(mask) = mask {
        legal.set_iterator_mask(mask);
    }

    let cap = legal.len();

    // Separate buckets: high = score > 0, quiet = score == 0
    let mut high = Vec::with_capacity(cap);
    let mut quiet = Vec::with_capacity(cap);

    for m in legal {
        // Try to find the move in the preferred list
        let mut move_score = 0;
        if let Some(preferred) = preferred {
            for &(pm, s) in preferred {
                if pm == m {
                    move_score = s;
                    break;
                }
            }
        }
        if move_score == 0 {
            move_score = score(m, board);
        }

        // bucket by score
        if move_score > 0 {
            high.push((m, move_score));
        } else {
            quiet.push((m, 0));
        }
    }

    // Sort only the forcing moves
    high.sort_unstable_by_key(|&(_, s)| -s);

    // Append quiet moves last
    high.extend(quiet);
    high
}

const PROMOTION_SCORE: i32 = 10000;
const PROMOTION_SCORE_QUEEN: i32 = PROMOTION_SCORE + 4;
const PROMOTION_SCORE_ROOK: i32 = PROMOTION_SCORE + 3;
const PROMOTION_SCORE_BISHOP: i32 = PROMOTION_SCORE + 2;
const PROMOTION_SCORE_KNIGHT: i32 = PROMOTION_SCORE + 1;

pub const CAPTURE_SCORE: i32 = 1000;

// MVV-LVA table
// king, queen, rook, bishop, knight, pawn
const MVV_LVA: [[i32; 6]; 6] = [
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

fn score(mov: ChessMove, board: &Board) -> i32 {
    // Check for promotions first
    if let Some(promotion) = mov.get_promotion() {
        return match promotion {
            Piece::Queen => PROMOTION_SCORE_QUEEN,
            Piece::Rook => PROMOTION_SCORE_ROOK,
            Piece::Bishop => PROMOTION_SCORE_BISHOP,
            Piece::Knight => PROMOTION_SCORE_KNIGHT,
            _ => 0,
        };
    }

    let attacker = board.piece_on(mov.get_source()).unwrap();
    let victim = board.piece_on(mov.get_dest());

    // Next look at captures (MVV-LVA)
    if let Some(victim) = victim {
        return CAPTURE_SCORE + MVV_LVA[mvva_lva_index(victim)][mvva_lva_index(attacker)];
    }

    // Quiet moves are not scored
    return 0;
}
