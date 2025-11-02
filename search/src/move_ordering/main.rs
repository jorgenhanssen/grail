// Move ordering for main search inspired by Black Marlin

use arrayvec::ArrayVec;
use chess::{BitBoard, Board, ChessMove, MoveGen, Piece, Square};
use evaluation::piece_values::PieceValues;

use crate::history::{CaptureHistory, ContinuationHistory, HistoryHeuristic};
use crate::utils::gives_check;
use crate::utils::see::see;

use super::utils::{capture_score, select_highest, ScoredMove};

pub const MAX_CAPTURES: usize = 32;
pub const MAX_QUIETS: usize = 96;

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

    good_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    bad_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    quiets: ArrayVec<ScoredMove, MAX_QUIETS>,

    piece_values: PieceValues,
    quiet_check_bonus: i16,
    threats: BitBoard,
}

impl MainMoveGenerator {
    pub fn new(
        best_move: Option<ChessMove>,
        killer_moves: [Option<ChessMove>; 2],
        prev_to: &[Option<Square>],
        game_phase: f32,
        piece_values: PieceValues,
        quiet_check_bonus: i16,
        threats: BitBoard,
    ) -> Self {
        Self {
            gen_phase: Phase::BestMove,
            game_phase,
            best_move,

            prev_to: prev_to.to_vec(),

            killer_moves,
            killer_index: 0,

            good_captures: ArrayVec::new(),
            bad_captures: ArrayVec::new(),
            quiets: ArrayVec::new(),

            piece_values,
            quiet_check_bonus,
            threats,
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

            for mov in gen.take(MAX_CAPTURES) {
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

                if scored_move.score < 0 {
                    self.bad_captures.push(scored_move);
                    continue;
                }

                // Use MVV-LVA for quick filtering before expensive SEE
                let victim = board.piece_on(scored_move.mov.get_dest()).unwrap();
                let attacker = board.piece_on(scored_move.mov.get_source()).unwrap();
                let victim_value = self.piece_values.get(victim, self.game_phase);
                let attacker_value = self.piece_values.get(attacker, self.game_phase);

                // If victim is more valuable than attacker, it's likely good - skip SEE
                if victim_value > attacker_value {
                    return Some(scored_move.mov);
                }

                // Only run expensive SEE if capture seems questionable
                if see(board, scored_move.mov, self.game_phase, &self.piece_values) < 0 {
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
                    // Skip if it's a capture (already searched in capture phases)
                    if board.piece_on(killer.get_dest()).is_some() {
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

            for mov in gen.take(MAX_QUIETS) {
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
                            self.threats,
                        );

                        let cont = continuation_history.get(
                            board.side_to_move(),
                            &self.prev_to,
                            mov.get_source(),
                            mov.get_dest(),
                        );

                        let check_bonus = if gives_check(board, mov) {
                            self.quiet_check_bonus
                        } else {
                            0
                        };

                        hist + cont + check_bonus
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
