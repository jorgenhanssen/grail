use crate::scores::{MATE_VALUE, SCORE_INF};
use arrayvec::ArrayVec;
use cozy_chess::{Board, Color, Move, Piece, Rank};
use utils::{QUEEN_VALUE, make_move, piece_value};

use utils::Node;

use crate::{
    MAX_DEPTH,
    move_ordering::QMoveGenerator,
    transposition::Bound,
    utils::{Bounds, see::see},
};

use super::{Searcher, pruning::mate_distance_prune};

impl Searcher {
    /// Quiescence search: continues searching captures until the position is stable enough
    /// for a reliable static evaluation.
    ///
    /// <https://www.chessprogramming.org/Quiescence_Search>
    pub(super) fn quiescence_search(&mut self, node: &Node, mut bounds: Bounds, ply: u8) -> i16 {
        self.pv_table.init_ply(ply);

        // Check if we should stop searching
        if self.shared.is_stopped() {
            return 0;
        }

        self.increment_nodes();
        self.max_ply_reached = self.max_ply_reached.max(ply);

        if self.is_forced_draw(node) {
            return self.draw_value();
        }

        // Ply limit - return static eval if we've hit max ply
        if ply as usize >= MAX_DEPTH {
            let raw_eval = self.static_eval(node);
            let prev_moves = self.search_stack.prev_moves();
            return self
                .shared
                .correction()
                .adjust(node.board(), &prev_moves, raw_eval);
        }

        let hash = node.hash();
        if mate_distance_prune(&mut bounds, ply) {
            return bounds.alpha;
        }

        let board = node.board();
        let in_check = node.in_check();

        let original_bounds = bounds;

        let tt_info = self.shared.tt().probe(hash, ply);
        let tt_pv = node.is_pv() || tt_info.is_some_and(|t| t.pv);
        if let Some(tt) = tt_info {
            if !node.is_pv() {
                match tt.bound {
                    Bound::Exact => return tt.value,
                    Bound::Lower if bounds.is_cutoff(tt.value) => return tt.value,
                    Bound::Upper if tt.value <= bounds.alpha => return tt.value,
                    _ => {}
                }
            }
        }

        // Reuse the cached NNUE result from the TT when available; the corrected
        // value is derived freshly so the cache stays semantically "raw eval".
        let static_eval = tt_info
            .and_then(|t| t.static_eval)
            .unwrap_or_else(|| self.static_eval(node));
        let prev_moves = self.search_stack.prev_moves();
        let stand_pat = self
            .shared
            .correction()
            .adjust(board, &prev_moves, static_eval);

        let board_material = node.total_material();

        // Delta pruning: skip captures that can't possibly improve alpha
        // https://www.chessprogramming.org/Delta_Pruning
        let can_delta =
            !in_check && board_material >= self.config.qs_delta_material_threshold.value;

        // Do a "stand-pat" evaluation if not in check
        if !in_check {
            if bounds.is_cutoff(stand_pat) {
                self.shared.tt().store(
                    hash,
                    ply,
                    0, // Prefer deeper entries rather than QS entries
                    stand_pat,
                    Some(static_eval),
                    original_bounds.alpha,
                    original_bounds.beta,
                    None,
                    tt_pv,
                );
                return stand_pat;
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
                    self.shared.tt().store(
                        hash,
                        ply,
                        0, // Prefer deeper entries rather than QS entries
                        stand_pat,
                        Some(static_eval),
                        original_bounds.alpha,
                        original_bounds.beta,
                        None,
                        tt_pv,
                    );
                    return stand_pat;
                }
            }

            bounds.raise_alpha(stand_pat);
        }

        let mut best_eval = if in_check { -SCORE_INF } else { stand_pat };
        let mut captures_searched: ArrayVec<Move, 32> = ArrayVec::new();

        let mut moves = QMoveGenerator::new(node, &self.capture_history);

        while let Some(mv) = moves.next() {
            // Per-move delta pruning (skip if capture can't possibly improve alpha)
            if can_delta {
                let captured = board.piece_on(mv.to);
                if let Some(piece) = captured {
                    let mut delta = piece_value(piece) + self.config.qs_delta_margin.value;
                    if let Some(promotion) = mv.promotion {
                        delta += piece_value(promotion) - piece_value(Piece::Pawn);
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

            self.shared.tt().prefetch(child_hash);

            let child = Node::new(new_board, node.node_type());

            self.search_stack.push_node(&child);
            let child_score = self.quiescence_search(&child, bounds.invert(), ply + 1);
            self.search_stack.pop();

            let value = -child_score;

            // Check if we were stopped during the recursive search
            if self.shared.is_stopped() {
                break;
            }

            if value > best_eval {
                best_eval = value;
                self.pv_table.update(ply, mv);
                bounds.raise_alpha(best_eval);
            }

            if bounds.is_cutoff(bounds.alpha) {
                self.on_qs_fail_high(board, mv, &captures_searched);
                break;
            }

            let _ = captures_searched.try_push(mv);
        }

        // If in check and no legal moves improved the position, it's checkmate
        if in_check && best_eval == -SCORE_INF {
            return -(MATE_VALUE - ply as i16);
        }

        self.shared.tt().store(
            hash,
            ply,
            0,
            best_eval,
            Some(static_eval),
            original_bounds.alpha,
            original_bounds.beta,
            None,
            tt_pv,
        );
        best_eval
    }

    /// Updates capture history on a QS beta cutoff. Uses depth 1 to keep
    /// bonus/malus small relative to main search.
    fn on_qs_fail_high(&mut self, board: &Board, mv: Move, captures_searched: &[Move]) {
        let bonus = self.capture_history.get_bonus(1);
        self.capture_history.update_capture(board, mv, bonus);

        let malus = self.capture_history.get_malus(1);
        for &c in captures_searched {
            self.capture_history.update_capture(board, c, malus);
        }
    }
}
