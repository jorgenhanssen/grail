// Move ordering inspired by Black Marlin

use chess::{Board, ChessMove, MoveGen, Piece, Square};
use evaluation::piece_values::PieceValues;

use crate::utils::{see, CaptureHistory, ContinuationHistory, HistoryHeuristic};

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

    // Continuation history context
    prev_to: Vec<Option<Square>>,

    killer_moves: [Option<ChessMove>; 2],
    killer_index: usize,

    good_captures: Vec<ScoredMove>,
    bad_captures: Vec<ScoredMove>,
    quiets: Vec<ScoredMove>,

    piece_values: PieceValues,
}

impl MainMoveGenerator {
    pub fn new(
        best_move: Option<ChessMove>,
        killer_moves: [Option<ChessMove>; 2],
        prev_to: &[Option<Square>],
        game_phase: f32,
        piece_values: PieceValues,
    ) -> Self {
        Self {
            gen_phase: Phase::BestMove,
            game_phase,
            best_move,

            prev_to: prev_to.to_vec(),

            killer_moves,
            killer_index: 0,

            good_captures: Vec::new(),
            bad_captures: Vec::new(),
            quiets: Vec::new(),

            piece_values,
        }
    }

    pub fn next(
        &mut self,
        board: &Board,
        history_heuristic: &HistoryHeuristic,
        capture_history: &CaptureHistory,
        continuation_history: &ContinuationHistory,
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
                    score: capture_score(
                        board,
                        mov,
                        capture_history,
                        self.game_phase,
                        &self.piece_values,
                    ),
                });
            }
        }

        if self.gen_phase == Phase::GoodCaptures {
            while let Some(index) = select_highest(&self.good_captures) {
                let scored_move = self.good_captures.swap_remove(index);

                if scored_move.score < 0
                    || see(board, scored_move.mov, self.game_phase, &self.piece_values) < 0
                {
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

                        let cont = continuation_history.get(
                            board.side_to_move(),
                            &self.prev_to,
                            mov.get_source(),
                            mov.get_dest(),
                        );

                        hist + cont
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
    pub fn new(
        in_check: bool,
        board: &Board,
        capture_history: &CaptureHistory,
        phase: f32,
        piece_values: PieceValues,
    ) -> Self {
        let mut gen = MoveGen::new_legal(board);

        if !in_check {
            gen.set_iterator_mask(*board.color_combined(!board.side_to_move()));

            let mut forcing_moves = vec![];

            for mov in gen {
                forcing_moves.push(ScoredMove {
                    mov,
                    score: capture_score(board, mov, capture_history, phase, &piece_values),
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

// Replacement scoring for captures using Capture History.
#[inline(always)]
fn capture_score(
    board: &Board,
    mv: ChessMove,
    capture_history: &CaptureHistory,
    phase: f32,
    piece_values: &PieceValues,
) -> i16 {
    let victim = board.piece_on(mv.get_dest()).unwrap();
    let attacker = board.piece_on(mv.get_source()).unwrap();
    let hist = capture_history.get(attacker, mv.get_dest(), victim);

    piece_values.get(victim, phase) + hist
}
