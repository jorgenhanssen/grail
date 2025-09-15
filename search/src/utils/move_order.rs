// Move ordering inspired by Black Marlin

use chess::{Board, ChessMove, MoveGen, Piece};

use crate::utils::{see, HistoryHeuristic};

struct ScoredMove {
    mov: ChessMove,
    score: i16,
}

#[derive(PartialEq, Eq, Clone)]
enum Phase {
    BestMove,
    GenCaptures,
    GoodCaptures,
    GenQuiets,
    Killers,
    Quiets,
    BadCaptures,
}

pub struct MainMoveGenerator {
    gen_phase: Phase,
    game_phase: f32,

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
        game_phase: f32,
    ) -> Self {
        Self {
            gen_phase: Phase::BestMove,
            game_phase,
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
        if self.gen_phase == Phase::BestMove {
            self.gen_phase = Phase::GenCaptures;
            if let Some(best_move) = self.best_move {
                if board.legal(best_move) {
                    return Some(best_move);
                }
            }
        }

        if self.gen_phase == Phase::GenCaptures {
            self.gen_phase = Phase::GoodCaptures;

            let mut gen = MoveGen::new_legal(board);
            let capture_mask = board.color_combined(!board.side_to_move());
            gen.set_iterator_mask(*capture_mask);

            for mov in gen {
                if Some(mov) == self.best_move {
                    continue;
                }

                self.good_captures.push(ScoredMove {
                    mov,
                    score: capture_score(board, mov),
                });
            }
        }

        if self.gen_phase == Phase::GoodCaptures {
            while let Some(index) = select_highest(&self.good_captures) {
                let scored_move = self.good_captures.swap_remove(index);

                if see(board, scored_move.mov, self.game_phase) < 0 {
                    self.bad_captures.push(scored_move);
                    continue;
                }

                return Some(scored_move.mov);
            }
            self.gen_phase = Phase::Killers;
        }

        if self.gen_phase == Phase::Killers {
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
            self.gen_phase = Phase::GenQuiets;
        }

        if self.gen_phase == Phase::GenQuiets {
            self.gen_phase = Phase::Quiets;

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
                            1000
                        } else {
                            0
                        };

                        hist + counter
                    }
                };

                self.quiets.push(ScoredMove { mov, score });
            }
        }

        if self.gen_phase == Phase::Quiets {
            if let Some(index) = select_highest(&self.quiets) {
                let scored_move = self.quiets.swap_remove(index);
                return Some(scored_move.mov);
            }
            self.gen_phase = Phase::BadCaptures;
        }

        if self.gen_phase == Phase::BadCaptures {
            if let Some(index) = select_highest(&self.bad_captures) {
                let scored_move = self.bad_captures.swap_remove(index);
                return Some(scored_move.mov);
            }
        }

        None
    }
}

pub struct QMoveGenerator {
    forcing_moves: Vec<ScoredMove>,
}

impl QMoveGenerator {
    pub fn new(in_check: bool, board: &Board) -> Self {
        let mut gen = MoveGen::new_legal(board);

        if !in_check {
            gen.set_iterator_mask(*board.color_combined(!board.side_to_move()));

            let mut forcing_moves = vec![];

            for mov in gen {
                forcing_moves.push(ScoredMove {
                    mov,
                    score: capture_score(board, mov),
                });
            }

            Self { forcing_moves }
        } else {
            Self {
                forcing_moves: gen.map(|mov| ScoredMove { mov, score: 0 }).collect(),
            }
        }
    }

    pub fn next(&mut self) -> Option<ChessMove> {
        if let Some(index) = select_highest(&self.forcing_moves) {
            let scored_move = self.forcing_moves.swap_remove(index);
            return Some(scored_move.mov);
        }
        None
    }
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

#[inline(always)]
fn capture_score(board: &Board, mv: ChessMove) -> i16 {
    let victim = board.piece_on(mv.get_dest()).unwrap();
    let attacker = board.piece_on(mv.get_source()).unwrap();
    MVV_LVA[mvva_lva_index(victim)][mvva_lva_index(attacker)]
}

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
#[inline(always)]
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
