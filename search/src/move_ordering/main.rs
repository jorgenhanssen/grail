use arrayvec::ArrayVec;
use cozy_chess::{BitBoard, Move, Piece, Square};
use utils::{gives_check, is_capture, piece_value};

use crate::history::{CaptureHistory, ContinuationHistory, HistoryHeuristic};
use crate::utils::see::see;
use utils::Node;

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

/// Staged move generator for main search. Based on Black Marlin.
///
/// Generates and sorts moves lazily in phases to avoid doing it all upfront:
/// 1. BestMove (TT/PV move) - most likely to cause cutoff
/// 2. GoodCaptures - winning/equal captures by SEE (includes capture promotions)
/// 3. Killers - quiet moves that caused cutoffs at this ply before
/// 4. Quiets - remaining quiet moves, scored by history (queen promos first, underpromos last)
/// 5. BadCaptures - losing captures, tried last
///
/// <https://www.chessprogramming.org/Move_Ordering>
/// <https://www.chessprogramming.org/Killer_Heuristic>
/// <https://github.com/jnlt3/blackmarlin>
pub struct MainMoveGenerator {
    gen_phase: Phase,

    best_move: Option<Move>,

    // Continuation history context
    prev_to: Vec<Option<Square>>,

    killer_moves: [Option<Move>; 2],
    killer_index: usize,

    good_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    bad_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    quiets: ArrayVec<ScoredMove, MAX_QUIETS>,

    quiet_check_bonus: i16,
    threats: BitBoard,
}

impl MainMoveGenerator {
    pub fn new(
        best_move: Option<Move>,
        killer_moves: [Option<Move>; 2],
        prev_to: Vec<Option<Square>>,
        quiet_check_bonus: i16, // Re-export Node and NodeType from utils for convenience
        threats: BitBoard,
    ) -> Self {
        Self {
            gen_phase: Phase::BestMove,
            best_move,

            prev_to,

            killer_moves,
            killer_index: 0,

            good_captures: ArrayVec::new(),
            bad_captures: ArrayVec::new(),
            quiets: ArrayVec::new(),

            quiet_check_bonus,
            threats,
        }
    }

    pub fn next(
        &mut self,
        node: &Node,
        history_heuristic: &HistoryHeuristic,
        capture_history: &CaptureHistory,
        continuation_history: &ContinuationHistory,
    ) -> Option<Move> {
        let board = node.board();

        if self.gen_phase == Phase::BestMove {
            self.gen_phase = Phase::GenCaptures;
            if let Some(best_move) = self.best_move {
                if board.is_legal(best_move) {
                    return Some(best_move);
                }
            }
        }

        if self.gen_phase == Phase::GenCaptures {
            self.gen_phase = Phase::GoodCaptures;

            let enemy_pieces = board.colors(!board.side_to_move());

            board.generate_moves(|moves| {
                let mut captures = moves;
                captures.to &= enemy_pieces;

                for mov in captures {
                    if Some(mov) == self.best_move {
                        continue;
                    }
                    if self.good_captures.len() >= MAX_CAPTURES {
                        return true;
                    }

                    self.good_captures.push(ScoredMove {
                        mov,
                        score: capture_score(board, mov, capture_history),
                    });
                }
                false
            });
        }

        if self.gen_phase == Phase::GoodCaptures {
            while let Some(index) = select_highest(&self.good_captures) {
                let scored_move = self.good_captures.swap_remove(index);

                if scored_move.score < 0 {
                    self.bad_captures.push(scored_move);
                    continue;
                }

                // Use MVV-LVA for quick filtering before expensive SEE
                let victim = board.piece_on(scored_move.mov.to).unwrap();
                let attacker = board.piece_on(scored_move.mov.from).unwrap();
                let victim_value = piece_value(victim);
                let attacker_value = piece_value(attacker);

                // If victim is more valuable than attacker, it's likely good - skip SEE
                if victim_value > attacker_value {
                    return Some(scored_move.mov);
                }

                // Only run expensive SEE if capture seems questionable
                if !see(board, scored_move.mov, 0) {
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
                    if !board.is_legal(killer) {
                        continue;
                    }
                    // Skip if it's a capture (already searched in capture phases)
                    if is_capture(board, killer) {
                        continue;
                    }
                    return Some(killer);
                }
            }
            self.gen_phase = Phase::GenQuiets;
        }

        if self.gen_phase == Phase::GenQuiets {
            self.gen_phase = Phase::Quiets;

            let empty_squares = !board.occupied();
            let our_pieces = board.colors(board.side_to_move());

            board.generate_moves(|moves| {
                for mov in moves {
                    // Allow moves to empty squares OR castling (king captures own rook in cozy-chess)
                    let is_castling = our_pieces.has(mov.to);
                    if !empty_squares.has(mov.to) && !is_castling {
                        continue;
                    }
                    if Some(mov) == self.best_move {
                        continue;
                    }
                    if self.killer_moves.contains(&Some(mov)) {
                        continue;
                    }
                    if self.quiets.len() >= MAX_QUIETS {
                        return true;
                    }

                    let score = match mov.promotion {
                        Some(Piece::Queen) => i16::MAX,
                        Some(_) => i16::MIN,
                        None => {
                            let hist = history_heuristic.get(
                                board.side_to_move(),
                                mov.from,
                                mov.to,
                                self.threats,
                            );

                            let cont = continuation_history.get(
                                board.side_to_move(),
                                &self.prev_to,
                                mov.from,
                                mov.to,
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
                false
            });
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
