use chess::{Board, ChessMove, MoveGen, Piece};

pub fn get_ordered_moves(
    board: &Board,
    preferred: Option<&[(ChessMove, i32)]>,
) -> Vec<(ChessMove, i32)> {
    let legal = MoveGen::new_legal(board);
    let cap   = legal.len();

    // Separate buckets: high = score > 0, quiet = score == 0
    let mut high  = Vec::with_capacity(cap);
    let mut quiet = Vec::with_capacity(cap);

    let (pref_ptr, pref_len) = if let Some(p) = preferred {
        (p.as_ptr(), p.len()) // raw ptr so the compiler can unroll
    } else {
        (core::ptr::null(), 0)
    };

    // ------------ main loop ------------
    for m in legal {
        // ---- tiny hand scan of preferred (k <= 4) ----
        let mut sc = 0;
        if pref_len != 0 {
            // SAFETY: pref_ptr came from a valid slice, len is correct
            for i in 0..pref_len {
                // LLVM can vectorise / unroll this
                let (pm, s) = unsafe { *pref_ptr.add(i) };
                if pm == m {
                    sc = s;
                    break;
                }
            }
        }
        if sc == 0 {
            sc = score(m, board); // your existing scoring fn
        }

        // ---- bucket by score ----
        if sc > 0 {
            high.push((m, sc));
        } else {
            quiet.push((m, 0));
        }
    }

    // Sort only the forcing moves (usually a small subset)
    high.sort_unstable_by_key(|&(_, s)| -s);

    // Append the quiet bucket without sorting
    high.extend(quiet);
    high
}

pub const PROMOTION_SCORE: i32 = 10000;
const PROMOTION_SCORE_QUEEN: i32 = PROMOTION_SCORE + 4;
const PROMOTION_SCORE_ROOK: i32 = PROMOTION_SCORE + 3;
const PROMOTION_SCORE_BISHOP: i32 = PROMOTION_SCORE + 2;
const PROMOTION_SCORE_KNIGHT: i32 = PROMOTION_SCORE + 1;

pub const CAPTURE_SCORE: i32 = 1000;

pub const CHECK_SCORE: i32 = 100;

pub const PIECE_SCORE: i32 = 10;
const PIECE_SCORE_QUEEN: i32 = PIECE_SCORE + 6;
const PIECE_SCORE_ROOK: i32 = PIECE_SCORE + 5;
const PIECE_SCORE_BISHOP: i32 = PIECE_SCORE + 4;
const PIECE_SCORE_KNIGHT: i32 = PIECE_SCORE + 3;
const PIECE_SCORE_PAWN: i32 = PIECE_SCORE + 2;
const PIECE_SCORE_KING: i32 = PIECE_SCORE + 1;

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

    return 0;
}
