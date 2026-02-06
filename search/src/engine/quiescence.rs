use std::sync::atomic::Ordering;

use cozy_chess::{Color, Move, Piece, Rank};
use evaluation::scores::{MATE_VALUE, SCORE_INF};
use utils::{QUEEN_VALUE, make_move, piece_value};

use utils::Node;

use crate::{
    MAX_DEPTH,
    move_ordering::QMoveGenerator,
    pruning::{can_delta_prune, mate_distance_prune},
    transposition::Bound,
    utils::{Bounds, see::see},
};

use super::Engine;

impl Engine {
    /// Quiescence search: continues searching captures until the position is stable enough
    /// for a reliable static evaluation.
    ///
    /// <https://www.chessprogramming.org/Quiescence_Search>
    pub(super) fn quiescence_search(
        &mut self,
        node: &Node,
        mut bounds: Bounds,
        ply: u8,
    ) -> (i16, Vec<Move>) {
        // Check if we should stop searching
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }

        self.nodes += 1;
        self.max_ply_reached = self.max_ply_reached.max(ply);

        if self.is_forced_draw(node) {
            return (self.draw_value(), Vec::new());
        }

        // Ply limit - return static eval if we've hit max ply
        if ply as usize >= MAX_DEPTH {
            return (self.corrected_static_eval(node), Vec::new());
        }

        let hash = node.hash();
        if mate_distance_prune(&mut bounds, ply) {
            return (bounds.alpha, Vec::new());
        }

        let board = node.board();
        let in_check = node.in_check();

        let original_bounds = bounds;

        // QS entries don't track depth. All quiescence searches explore the same
        // tactical horizon, so any hit is trustworthy for cutoffs
        if let Some(tt) = self.qs_tt.probe(hash, in_check) {
            match tt.bound {
                Bound::Exact => return (tt.value, Vec::new()),
                Bound::Lower if bounds.is_cutoff(tt.value) => return (tt.value, Vec::new()),
                Bound::Upper if tt.value <= bounds.alpha => return (tt.value, Vec::new()),
                _ => {}
            }
        }

        let stand_pat = self.corrected_static_eval(node);

        let board_material = node.total_material();

        let can_delta = can_delta_prune(
            in_check,
            self.config.qs_delta_material_threshold.value,
            board_material,
        );

        // Do a "stand-pat" evaluation if not in check
        if !in_check {
            if bounds.is_cutoff(stand_pat) {
                self.qs_tt.store(
                    hash,
                    stand_pat,
                    original_bounds.alpha,
                    original_bounds.beta,
                    in_check,
                );
                return (stand_pat, Vec::new());
            }

            // Node-level delta pruning (big delta)
            if can_delta {
                let mut big_delta = QUEEN_VALUE;
                let promotion_rank = if board.side_to_move() == Color::White {
                    Rank::Seventh
                } else {
                    Rank::Second
                };
                let pawns = board.colored_pieces(board.side_to_move(), Piece::Pawn);
                let promoting_pawns = pawns & promotion_rank.bitboard();

                if !promoting_pawns.is_empty() {
                    big_delta += QUEEN_VALUE - piece_value(Piece::Pawn);
                }

                if stand_pat + big_delta < bounds.alpha {
                    self.qs_tt.store(
                        hash,
                        stand_pat,
                        original_bounds.alpha,
                        original_bounds.beta,
                        in_check,
                    );
                    return (stand_pat, Vec::new());
                }
            }

            bounds.raise_alpha(stand_pat);
        }

        let mut best_line = Vec::new();
        let mut best_eval = if in_check { -SCORE_INF } else { stand_pat };

        let mut moves = QMoveGenerator::new(node, &self.capture_history);

        while let Some(mv) = moves.next() {
            // Per-move delta pruning (skip if capture can't possibly improve alpha)
            if can_delta {
                let captured = board.piece_on(mv.to);
                if let Some(piece) = captured {
                    let mut delta = piece_value(piece) + self.config.qs_delta_margin.value;
                    if let Some(promotion) = mv.promotion {
                        delta += piece_value(promotion) - piece_value(Piece::Pawn);
                        // promotion bonus
                    }
                    if stand_pat + delta < bounds.alpha {
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
                        let victim_value = piece_value(victim);
                        let attacker_value = piece_value(attacker);
                        // Only run expensive SEE if capture seems questionable (equal/lower value)
                        if victim_value <= attacker_value && !see(board, mv, 0) {
                            continue;
                        }
                    }
                }
            }

            let new_board = make_move(board, mv);
            let child_hash = new_board.hash();

            self.qs_tt.prefetch(child_hash);

            let child = Node::new(new_board, node.node_type());

            self.search_stack.push_node(&child);
            let (child_score, mut child_line) =
                self.quiescence_search(&child, bounds.invert(), ply + 1);
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
                bounds.raise_alpha(best_eval);
            }

            if bounds.is_cutoff(bounds.alpha) {
                break; // Beta cutoff
            }
        }

        // If in check and no legal moves improved the position, it's checkmate
        if in_check && best_eval == -SCORE_INF {
            return (-(MATE_VALUE - ply as i16), Vec::new());
        }

        self.qs_tt.store(
            hash,
            best_eval,
            original_bounds.alpha,
            original_bounds.beta,
            in_check,
        );
        (best_eval, best_line)
    }
}
