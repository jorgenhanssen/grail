use std::sync::atomic::Ordering;

use cozy_chess::{Board, Color, Move, Piece, Rank};
use evaluation::scores::{MATE_VALUE, SCORE_INF};
use utils::flip_eval_perspective;
use utils::{game_phase, has_check, make_move, Position};

use crate::{
    move_ordering::QMoveGenerator,
    pruning::{can_delta_prune, mate_distance_prune},
    stack::SearchNode,
    transposition::Bound,
    utils::see::see,
};

use super::Engine;

impl Engine {
    /// Quiescence search: continues searching captures until the position is stable enough
    /// for a reliable static evaluation.
    ///
    /// <https://www.chessprogramming.org/Quiescence_Search>
    pub(super) fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: i16,
        mut beta: i16,
        depth: u8,
    ) -> (i16, Vec<Move>) {
        // Check if we should stop searching
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }

        self.nodes += 1;
        self.max_depth_reached = self.max_depth_reached.max(depth);

        // If this position has been seen before, treat it as a draw
        if self.search_stack.is_repetition(&self.game_history) {
            return (0, Vec::new());
        }

        let hash = self.search_stack.current().hash;
        if mate_distance_prune(&mut alpha, &mut beta, depth) {
            return (alpha, Vec::new());
        }

        let in_check = has_check(board);

        let original_alpha = alpha;
        let original_beta = beta;

        // QS entries don't track depth. All quiescence searches explore the same
        // tactical horizon, so any hit is trustworthy for cutoffs
        if let Some(tt) = self.qs_tt.probe(hash, in_check) {
            match tt.bound {
                Bound::Exact => return (tt.value, Vec::new()),
                Bound::Lower if tt.value >= beta => return (tt.value, Vec::new()),
                Bound::Upper if tt.value <= alpha => return (tt.value, Vec::new()),
                _ => {}
            }
        }

        let phase = game_phase(board);
        let position = Position::new(board);

        let eval = self.eval(&position, phase);
        let stand_pat = flip_eval_perspective(board.side_to_move(), eval);

        // Do a "stand-pat" evaluation if not in check
        if !in_check {
            if stand_pat >= beta {
                self.qs_tt
                    .store(hash, stand_pat, original_alpha, original_beta, in_check);
                return (stand_pat, Vec::new());
            }

            let total_material = self.piece_values.total_material(board, phase);

            // Node-level delta pruning (big delta)
            if can_delta_prune(
                in_check,
                self.config.qs_delta_material_threshold.value,
                total_material,
            ) {
                let mut big_delta = self.piece_values.get(Piece::Queen, phase);
                let promotion_rank = if board.side_to_move() == Color::White {
                    Rank::Seventh
                } else {
                    Rank::Second
                };
                let pawns = board.colored_pieces(board.side_to_move(), Piece::Pawn);
                let promoting_pawns = pawns & promotion_rank.bitboard();

                if !promoting_pawns.is_empty() {
                    big_delta += self.piece_values.get(Piece::Queen, phase)
                        - self.piece_values.get(Piece::Pawn, phase);
                }

                if stand_pat + big_delta < alpha {
                    self.qs_tt
                        .store(hash, stand_pat, original_alpha, original_beta, in_check);
                    return (stand_pat, Vec::new());
                }
            }

            alpha = alpha.max(stand_pat);
        }

        let mut best_line = Vec::new();
        let mut best_eval = if in_check { -SCORE_INF } else { stand_pat };

        let mut moves = QMoveGenerator::new(
            in_check,
            board,
            &self.capture_history,
            phase,
            self.piece_values,
        );

        while let Some(mv) = moves.next() {
            // Per-move delta pruning (skip if capture can't possibly improve alpha)
            if can_delta_prune(
                in_check,
                self.config.qs_delta_material_threshold.value,
                self.piece_values.total_material(board, phase),
            ) {
                let captured = board.piece_on(mv.to);
                if let Some(piece) = captured {
                    let mut delta =
                        self.piece_values.get(piece, phase) + self.config.qs_delta_margin.value;
                    if let Some(promotion) = mv.promotion {
                        delta += self.piece_values.get(promotion, phase)
                            - self.piece_values.get(Piece::Pawn, phase);
                        // promotion bonus
                    }
                    if stand_pat + delta < alpha {
                        continue;
                    }
                } else {
                    // Not a capture (should not happen with mask, but skip for safety)
                    continue;
                }
            }

            // Use MVV-LVA for quick pruning before expensive SEE
            if !in_check {
                if let Some(victim) = board.piece_on(mv.to) {
                    if let Some(attacker) = board.piece_on(mv.from) {
                        let victim_value = self.piece_values.get(victim, phase);
                        let attacker_value = self.piece_values.get(attacker, phase);
                        // Only run expensive SEE if capture seems questionable (equal/lower value)
                        if victim_value <= attacker_value
                            && !see(board, mv, phase, &self.piece_values, 0)
                        {
                            continue;
                        }
                    }
                }
            }

            let new_board = make_move(board, mv);
            let child_hash = new_board.hash();

            // Prefetch QS TT entry to hide memory latency
            self.qs_tt.prefetch(child_hash);

            self.search_stack.push(SearchNode::new(child_hash));
            let (child_score, mut child_line) =
                self.quiescence_search(&new_board, -beta, -alpha, depth + 1);
            self.search_stack.pop();

            let value = -child_score;

            // Check if we were stopped during the recursive search
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            if value > best_eval {
                best_eval = value;
                child_line.insert(0, mv);
                best_line = child_line;
                alpha = alpha.max(best_eval);
            }

            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        // If in check and no legal moves improved the position, it's checkmate
        if in_check && best_eval == -SCORE_INF {
            return (-(MATE_VALUE - depth as i16), Vec::new());
        }

        self.qs_tt
            .store(hash, best_eval, original_alpha, original_beta, in_check);
        (best_eval, best_line)
    }
}
