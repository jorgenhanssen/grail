use arrayvec::ArrayVec;
use cozy_chess::{BitBoard, Move, Piece};
use utils::{PAWN_VALUE, captured_piece, gives_check, piece_value};

use crate::history::{CaptureHistory, ContinuationHistory, HistoryHeuristic, PieceTo};
use crate::utils::see::see;
use utils::Node;

use super::utils::{ScoredMove, capture_score, select_highest};

pub const MAX_CAPTURES: usize = 32;
pub const MAX_QUIETS: usize = 96;

#[derive(PartialEq, Eq, Clone)]
enum Phase {
    BestMove,
    GenCaptures,
    GoodCaptures,
    GenQuiets,
    GoodQuiets,
    BadCaptures,
    BadQuiets,
}

/// Staged move generator for main search. Based on Black Marlin.
///
/// Generates and sorts moves lazily in phases to avoid doing it all upfront:
/// 1. BestMove (TT/PV move) - most likely to cause cutoff
/// 2. GoodCaptures - winning/equal captures by SEE (includes capture promotions)
/// 3. GoodQuiets - quiet moves with good score (queen promos first)
/// 4. BadCaptures - losing captures, tried late
/// 5. BadQuiets - quiet moves with bad score (underpromos last)
///
/// <https://www.chessprogramming.org/Move_Ordering>
/// <https://github.com/jnlt3/blackmarlin>
pub struct MainMoveGenerator {
    gen_phase: Phase,

    best_move: Option<Move>,

    // Continuation history context
    prev_moves: Vec<Option<PieceTo>>,

    good_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    bad_captures: ArrayVec<ScoredMove, MAX_CAPTURES>,
    good_quiets: ArrayVec<ScoredMove, MAX_QUIETS>,
    bad_quiets: ArrayVec<ScoredMove, MAX_QUIETS>,

    quiet_check_bonus: i16,
    quiet_check_see_margin: i16,
    bad_quiet_threshold: i16,
    escape_divisor: i16,
    unsafe_square_divisor: i16,
    threats: BitBoard,
    enemy_attacks: BitBoard,
}

impl MainMoveGenerator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        best_move: Option<Move>,
        prev_moves: Vec<Option<PieceTo>>,
        quiet_check_bonus: i16,
        quiet_check_see_margin: i16,
        bad_quiet_threshold: i16,
        escape_divisor: i16,
        unsafe_square_divisor: i16,
        threats: BitBoard,
        enemy_attacks: BitBoard,
    ) -> Self {
        Self {
            gen_phase: Phase::BestMove,
            best_move,

            prev_moves,

            good_captures: ArrayVec::new(),
            bad_captures: ArrayVec::new(),
            good_quiets: ArrayVec::new(),
            bad_quiets: ArrayVec::new(),

            quiet_check_bonus,
            quiet_check_see_margin,
            bad_quiet_threshold,
            escape_divisor,
            unsafe_square_divisor,
            threats,
            enemy_attacks,
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
                let victim = captured_piece(board, scored_move.mov).unwrap();
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
            self.gen_phase = Phase::GenQuiets;
        }

        if self.gen_phase == Phase::GenQuiets {
            self.gen_phase = Phase::GoodQuiets;

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
                    if self.good_quiets.len() + self.bad_quiets.len() >= MAX_QUIETS {
                        return true;
                    }

                    let score = match mov.promotion {
                        Some(Piece::Queen) => i16::MAX,
                        Some(_) => i16::MIN,
                        None => {
                            let mut score = 0;

                            let color = board.side_to_move();
                            let piece = board.piece_on(mov.from).unwrap();

                            score += history_heuristic.get(color, mov.from, mov.to, self.threats);

                            let curr = PieceTo::new(color, piece, mov.to);
                            score += continuation_history.get(&self.prev_moves, curr);

                            let value = piece_value(piece);
                            let to_unsafe = self.enemy_attacks.has(mov.to);

                            // Gives check = great (sometimes)
                            if gives_check(board, mov)
                                // Try to filter out "junk checks" that just hang material
                                && (!to_unsafe || see(board, mov, -self.quiet_check_see_margin))
                            {
                                score += self.quiet_check_bonus;
                            }

                            // Escapes threat = good
                            if self.threats.has(mov.from) {
                                score += value / self.escape_divisor;
                            }

                            // A valuable piece moving to an attacked square = bad
                            if to_unsafe && value > PAWN_VALUE {
                                score -= value / self.unsafe_square_divisor;
                            }

                            score
                        }
                    };

                    if score >= self.bad_quiet_threshold {
                        self.good_quiets.push(ScoredMove { mov, score });
                    } else {
                        self.bad_quiets.push(ScoredMove { mov, score });
                    }
                }
                false
            });
        }

        if self.gen_phase == Phase::GoodQuiets {
            if let Some(index) = select_highest(&self.good_quiets) {
                let scored_move = self.good_quiets.swap_remove(index);
                return Some(scored_move.mov);
            }
            self.gen_phase = Phase::BadCaptures;
        }

        if self.gen_phase == Phase::BadCaptures {
            if let Some(index) = select_highest(&self.bad_captures) {
                let scored_move = self.bad_captures.swap_remove(index);
                return Some(scored_move.mov);
            }
            self.gen_phase = Phase::BadQuiets;
        }

        if self.gen_phase == Phase::BadQuiets {
            if let Some(index) = select_highest(&self.bad_quiets) {
                let scored_move = self.bad_quiets.swap_remove(index);
                return Some(scored_move.mov);
            }
        }

        None
    }
}
