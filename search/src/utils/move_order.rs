// Move ordering inspired by Black Marlin

use chess::{BitBoard, Board, ChessMove, MoveGen, Piece};

use crate::utils::{game_phase, see, HistoryHeuristic};

#[derive(PartialEq, Eq, Copy, Debug, Clone, PartialOrd, Ord)]
enum Phase {
    BestMove,
    GenCaptures,
    GoodCaptures,
    GenQuiets,
    Killers,
    Quiets,
    BadCaptures,
}

struct ScoredMove {
    mv: ChessMove,
    score: i16,
}

fn select_highest(array: &[ScoredMove]) -> Option<usize> {
    if array.is_empty() {
        return None;
    }
    let mut best: Option<(i16, usize)> = None;
    for (index, mv) in array.iter().enumerate() {
        if let Some((best_score, _)) = best {
            if mv.score <= best_score {
                continue;
            }
        }
        best = Some((mv.score, index));
    }
    best.map(|(_, index)| index)
}

pub struct MainMoveGenerator {
    phase: Phase,

    best_move: Option<ChessMove>,

    countermove: Option<ChessMove>,

    killer_moves: [Option<ChessMove>; 2],
    killer_index: usize,

    good_captures: Vec<ScoredMove>,
    bad_captures: Vec<ScoredMove>,
    quiets: Vec<ScoredMove>,
}

impl MainMoveGenerator {
    pub fn new(
        best_move: Option<ChessMove>,
        killer_moves: [Option<ChessMove>; 2],
        countermove: Option<ChessMove>,
    ) -> Self {
        Self {
            phase: Phase::BestMove,
            best_move,

            countermove,

            killer_moves,
            killer_index: 0,

            good_captures: Vec::new(),
            bad_captures: Vec::new(),
            quiets: Vec::new(),
        }
    }

    pub fn next(
        &mut self,
        board: &Board,
        history_heuristic: &HistoryHeuristic,
    ) -> Option<ChessMove> {
        if self.phase == Phase::BestMove {
            self.phase = Phase::GenCaptures;
            if let Some(best_move) = self.best_move {
                return Some(best_move);
            }
        }

        if self.phase == Phase::GenCaptures {
            self.phase = Phase::GoodCaptures;

            let mut gen = MoveGen::new_legal(board);
            let capture_mask = board.color_combined(!board.side_to_move());
            gen.set_iterator_mask(*capture_mask);

            let phase = game_phase(board);
            for mov in gen {
                if Some(mov) == self.best_move {
                    continue;
                }
                let scored_move = ScoredMove {
                    mv: mov,
                    score: see(board, mov, phase),
                };
                if scored_move.score > 0 {
                    self.good_captures.push(scored_move);
                } else {
                    self.bad_captures.push(scored_move);
                }
            }
        }

        if self.phase == Phase::GoodCaptures {
            if let Some(index) = select_highest(&self.good_captures) {
                let scored_move = self.good_captures.swap_remove(index);
                return Some(scored_move.mv);
            }
            self.phase = Phase::Killers;
        }

        if self.phase == Phase::Killers {
            while self.killer_index < 2 {
                let killer = self.killer_moves[self.killer_index];
                self.killer_index += 1;
                if let Some(killer) = killer {
                    if Some(killer) == self.best_move {
                        continue;
                    }
                    if !board.legal(killer) {
                        continue;
                    }
                    return Some(killer);
                }
            }
            self.phase = Phase::GenQuiets;
        }

        if self.phase == Phase::GenQuiets {
            self.phase = Phase::Quiets;

            let mut gen = MoveGen::new_legal(board);
            gen.set_iterator_mask(!board.combined());

            for mov in gen {
                if Some(mov) == self.best_move {
                    continue;
                }
                if self.killer_moves.contains(&Some(mov)) {
                    continue;
                }

                let score = match mov.get_promotion() {
                    Some(Piece::Queen) => i16::MAX,
                    Some(_) => i16::MIN,
                    None => {
                        let hist = history_heuristic.get(
                            board.side_to_move(),
                            mov.get_source(),
                            mov.get_dest(),
                        );

                        let counter = if self.countermove == Some(mov) {
                            100
                        } else {
                            0
                        };

                        hist + counter
                    }
                };
                self.quiets.push(ScoredMove { mv: mov, score });
            }
        }

        if self.phase == Phase::Quiets {
            if let Some(index) = select_highest(&self.quiets) {
                let scored_move = self.quiets.swap_remove(index);
                return Some(scored_move.mv);
            }
            self.phase = Phase::BadCaptures;
        }
        if self.phase == Phase::BadCaptures {
            if let Some(index) = select_highest(&self.bad_captures) {
                let scored_move = self.bad_captures.swap_remove(index);
                return Some(scored_move.mv);
            }
        }

        None
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn ordered_moves(
    board: &Board,
    mask: Option<BitBoard>,
    depth: u8,
    best_move: Option<ChessMove>,
    countermove: Option<ChessMove>,
    killer_moves: &[[Option<ChessMove>; 2]],
    history_heuristic: &HistoryHeuristic,
) -> Vec<ChessMove> {
    let mut legal = MoveGen::new_legal(board);
    if let Some(mask) = mask {
        legal.set_iterator_mask(mask);
    }

    let mut moves_with_priority: Vec<(ChessMove, i32)> = Vec::with_capacity(64); // Rough estimate; chess max ~218

    let killers = &killer_moves[depth as usize];

    if let Some(_tt) = best_move {
        if board.legal(_tt) {
            moves_with_priority.push((_tt, MAX_PRIORITY + 1));
        }
    }

    for mov in legal {
        if Some(mov) == best_move {
            continue;
        }

        let mut priority = move_priority(&mov, board, history_heuristic);

        if Some(mov) == countermove {
            priority = priority.max(CAPTURE_PRIORITY - 2);
        }
        if killers.contains(&Some(mov)) {
            priority = priority.max(CAPTURE_PRIORITY - 1);
        }

        moves_with_priority.push((mov, priority));
    }

    moves_with_priority.sort_unstable_by_key(|&(_, p)| -p);

    moves_with_priority.into_iter().map(|(m, _)| m).collect()
}

// Piece moves get base priority (lowest)
pub const MIN_PRIORITY: i32 = 0;

// Captures get medium priority (MVV-LVA values 10-55)
pub const MIN_CAPTURE_PRIORITY: i32 = MIN_PRIORITY + 1_000_000;
pub const CAPTURE_PRIORITY: i32 = MIN_CAPTURE_PRIORITY;
// pub const MAX_CAPTURE_PRIORITY: i32 = MIN_CAPTURE_PRIORITY + 55;

// Promotions get highest priority
pub const MIN_PROMOTION_PRIORITY: i32 = MIN_PRIORITY + 2_000_000;
const UNDERPROMOTION_PRIORITY: i32 = MIN_PRIORITY - 3_000_000;
const PROMOTION_PRIORITY_QUEEN: i32 = MIN_PROMOTION_PRIORITY + 4;
pub const MAX_PROMOTION_PRIORITY: i32 = PROMOTION_PRIORITY_QUEEN;

pub const MAX_PRIORITY: i32 = MAX_PROMOTION_PRIORITY;

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

#[inline(always)]
fn move_priority(mov: &ChessMove, board: &Board, history_heuristic: &HistoryHeuristic) -> i32 {
    // Check for promotions first
    if let Some(promotion) = mov.get_promotion() {
        return match promotion {
            Piece::Queen => PROMOTION_PRIORITY_QUEEN,
            _ => UNDERPROMOTION_PRIORITY,
        };
    }

    let source = mov.get_source();
    let dest = mov.get_dest();

    let attacker = board.piece_on(source).unwrap();
    if let Some(victim) = board.piece_on(dest) {
        return CAPTURE_PRIORITY + MVV_LVA[mvva_lva_index(victim)][mvva_lva_index(attacker)];
    }

    let color = board.side_to_move();

    history_heuristic.get(color, source, dest) as i32
}
