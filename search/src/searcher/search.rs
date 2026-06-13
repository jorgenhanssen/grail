use std::sync::mpsc::Sender;

use crate::scores::{MATE_VALUE, SCORE_INF};
use arrayvec::ArrayVec;
use cozy_chess::{Move, Piece};
use uci::{
    UciOutput,
    commands::{GoParams, Info, Score},
};
use utils::{Node, NodeType, has_legal_moves};

use crate::{
    aspiration::Pass,
    history::{PieceTo, PrevMoves},
    move_ordering::{MAX_CAPTURES, MAX_QUIETS, MainMoveGenerator},
    pv::PvLine,
    result::SearchResult,
    stack::SearchNode,
    time_control::SearchController,
    transposition::{Bound, ProbeResult},
    utils::Bounds,
};

use super::{MAX_DEPTH, Searcher, pruning::mate_distance_prune};

impl Searcher {
    /// For main thread: Multi-PV search with iterative deepening, time control and UCI output.
    pub fn search(
        &mut self,
        params: &GoParams,
        output: Option<&Sender<UciOutput>>,
    ) -> Option<SearchResult> {
        // Check for checkmate (no legal moves when in check)
        if !has_legal_moves(&self.board) && !self.board.checkers().is_empty() {
            if let Some(output) = output {
                output
                    .send(UciOutput::Info(Info {
                        score: Score::Mate(0),
                        ..Default::default()
                    }))
                    .unwrap();
            }
            return None;
        }

        self.init_search();

        let mut controller =
            SearchController::new(params, &self.board, self.config.move_overhead.value as u64);
        self.deadline = controller.deadline();

        // Oh boy...
        // During datagen with softnodes (10k), several threads would hang
        // for an hour inside a single search. Best guess: tb/tt makes
        // a bunch of iterations cheap, depth climbs up, and then one
        // iteration is a bit more expensive and the deep iteration explodes.
        // So as a pragmatic failsafe I present a (generous) hard node limit,
        // and datagen has worked fine since I added it, so here it will stay.
        self.node_limit = params
            .nodes
            .or_else(|| params.soft_nodes.map(|n| n.saturating_mul(16)));

        let pv_count = self.config.multi_pv.value as usize;
        let root = Node::new(self.board.clone(), NodeType::Pv);
        let mut depth = 1u8;

        // Iterative deepening
        while !self.shared.is_stopped() && depth < MAX_DEPTH as u8 {
            controller.on_iteration_start();

            if !controller.should_continue_to_next_depth(depth) {
                break;
            }

            self.multi_pv.reset_excluded();
            self.root_depth = depth;

            for pv_index in 0..pv_count {
                let (pv, failures) = self.search_pv(&root, depth, pv_index);
                controller.add_aspiration_failures(failures);

                if let Some(pv) = pv {
                    // Exclude this move for subsequent PVs to not search it multiple times
                    if let Some(mv) = pv.best_move() {
                        self.multi_pv.add_excluded(mv);
                    }
                    if let Some(out) = output {
                        self.send_search_info(out, depth, &pv, controller.elapsed());
                    }
                    self.multi_pv.lines[pv_index].result = pv;
                } else {
                    break; // No more moves for additional PVs
                }

                if self.shared.is_stopped() {
                    break;
                }
            }

            if let Some(pv) = self.multi_pv.primary() {
                controller.on_iteration_complete(
                    depth,
                    pv.score,
                    pv.best_move(),
                    self.config.multi_pv.value,
                );
            }

            // Soft node limit: finish the current iteration, but don't start
            // a new one once the budget is spent.
            if let Some(soft_limit) = params.soft_nodes {
                if self.shared.total_nodes() + self.nodes >= soft_limit {
                    break;
                }
            }

            depth += 1;
        }

        // Collect all non-empty PV lines into the result
        let lines: Vec<PvLine> = self
            .multi_pv
            .lines
            .iter()
            .map(|ctx| ctx.result.clone())
            .filter(|pv| !pv.is_empty())
            .collect();

        if lines.is_empty() {
            None
        } else {
            Some(SearchResult::new(lines))
        }
    }

    /// For Lazy SMP: dumb search until stopped to populate the shared TT and correction history.
    ///
    /// <https://www.chessprogramming.org/Lazy_SMP>
    pub fn search_auxiliary(&mut self) {
        self.init_search();

        let root = Node::new(self.board.clone(), NodeType::Pv);
        let mut depth = 1u8;

        while !self.shared.is_stopped() && depth < MAX_DEPTH as u8 {
            // Each helper skips a different 1/3 of depths to seed TT diversity:
            //   Thread 1: skips depths 2, 5, 8, 11, ...
            //   Thread 2: skips depths 1, 4, 7, 10, ...
            if (depth as usize + self.thread_id).is_multiple_of(3) {
                depth += 1;
                continue;
            }

            let (pv, _) = self.search_pv(&root, depth, 0);
            if let Some(pv) = pv {
                self.multi_pv.lines[0].result = pv;
            }

            depth += 1;
        }
    }

    /// Aspiration window search for a single PV line.
    /// Returns the PV (if any) and the number of aspiration failures.
    fn search_pv(&mut self, root: &Node, depth: u8, pv_index: usize) -> (Option<PvLine>, u32) {
        self.multi_pv.begin_pv_search(pv_index, depth);

        let mut retries = 0;
        let mut failures = 0u32;

        loop {
            let bounds = self.multi_pv.lines[pv_index].window.bounds();

            let score = self.search_node(root, depth, 0, bounds, true);
            if self.pv_table.is_empty(0) {
                return (None, failures);
            }

            if self.shared.is_stopped() {
                return (
                    Some(PvLine::new(self.pv_table.get(0), score, pv_index)),
                    failures,
                );
            }

            match self.multi_pv.lines[pv_index].window.analyse_pass(score) {
                Pass::Hit(s) => {
                    return (
                        Some(PvLine::new(self.pv_table.get(0), s, pv_index)),
                        failures,
                    );
                }
                _ => {
                    failures += 1;
                    retries += 1;
                    if retries >= self.config.aspiration_window_retries.value {
                        self.multi_pv.lines[pv_index].window.fully_extend();
                        retries = 0;
                    }
                }
            }
        }
    }

    /// Initializes state for a new search.
    fn init_search(&mut self) {
        self.nodes = 0;
        self.max_ply_reached = 1;

        self.search_stack.clear();
        self.search_stack.push(SearchNode::new(self.board.hash()));

        self.multi_pv.init(
            self.config.multi_pv.value as usize,
            self.config.aspiration_window_size.value,
            self.config.aspiration_window_widen.value,
            self.config.aspiration_window_depth.value,
            self.config.aspiration_score_divisor.value,
        );
    }

    /// Alpha-beta search with principal variation search (PVS) and late move reductions.
    ///
    /// <https://www.chessprogramming.org/Principal_Variation_Search>
    pub(super) fn search_node(
        &mut self,
        node: &Node,
        depth: u8,
        ply: u8,
        mut bounds: Bounds,
        null_move_allowed: bool,
    ) -> i16 {
        self.pv_table.init_ply(ply);

        let singular = self.search_stack.current().and_then(|n| n.singular);

        self.check_limits();
        if self.shared.is_stopped() {
            return 0;
        }
        self.increment_nodes();

        if ply > 0 && self.is_forced_draw(node) {
            return self.draw_value();
        }

        if ply > 0 {
            if let Some(score) = self.probe_tb_wdl(node, depth) {
                return score;
            }
        }

        // As deep as we can go, so return static eval
        if ply as usize >= MAX_DEPTH {
            return self.static_eval(node);
        }

        if mate_distance_prune(&mut bounds, ply) {
            return bounds.alpha;
        }

        if depth == 0 {
            return self.quiescence_search(node, bounds, ply);
        }

        let hash = node.hash();
        let original_bounds = bounds;
        let is_pv_node = node.is_pv();

        let tt_info: Option<ProbeResult> = if let Some(tt) = self.shared.tt().probe(hash, ply) {
            // Only trust value/bound for cutoffs if the TT entry comes from a
            // search at least as deep as we need. Shallow results may have
            // missed tactics and can't safely prune the current search.
            //
            // Don't do TT cutoffs in PV nodes or during singular search.
            if !is_pv_node && singular.is_none() && tt.depth >= depth {
                match tt.bound {
                    // Exact: previous search found true minimax value
                    Bound::Exact => {
                        if let Some(m) = tt.best_move {
                            self.pv_table.set_move(ply, m);
                        }
                        return tt.value;
                    }
                    // Lower: previous search failed high (value >= beta), so value is at least this good
                    Bound::Lower => {
                        bounds.raise_alpha(tt.value);
                        if bounds.is_cutoff(bounds.alpha) {
                            if let Some(m) = tt.best_move {
                                self.pv_table.set_move(ply, m);
                            }
                            return tt.value;
                        }
                    }
                    // Upper: previous search failed low (value <= alpha), so value is at most this bad
                    Bound::Upper => {
                        bounds.beta = bounds.beta.min(tt.value);
                        if bounds.beta <= bounds.alpha {
                            if let Some(m) = tt.best_move {
                                self.pv_table.set_move(ply, m);
                            }
                            return bounds.beta;
                        }
                    }
                }
            }

            // Even if we are not able to return the TT move,
            // it is still valuable for cached static eval as hint for move ordering, etc.
            Some(tt)
        } else {
            None
        };

        let in_check = node.in_check();
        let tt_move = tt_info.and_then(|t| t.best_move);
        let tt_move_is_capture = tt_move.is_some_and(|m| node.is_capture(m));
        let tt_pv = is_pv_node || tt_info.is_some_and(|t| t.pv);

        let static_eval = tt_info
            .and_then(|t| t.static_eval)
            .unwrap_or_else(|| self.static_eval(node));

        let prev_moves = self.search_stack.prev_moves();

        let corrected_eval =
            self.shared
                .correction()
                .adjust(node.board(), &prev_moves, static_eval);

        // Prefer the TT score over static eval so trend sees what search already found.
        let stack_eval = tt_info
            .map(|tt| match tt.bound {
                Bound::Exact => tt.value,
                Bound::Lower if tt.value > corrected_eval => tt.value,
                Bound::Upper if tt.value < corrected_eval => tt.value,
                _ => corrected_eval,
            })
            .unwrap_or(corrected_eval);

        self.search_stack.current_mut(|n| n.eval = Some(stack_eval));

        // Stockfish skips NMP during singular searches.
        // Likely because NMP can raise beta without searching any actual moves,
        // so the "are all other moves worse?" test becomes unreliable.
        // I also saw success skipping razoring.
        if singular.is_none() {
            if let Some(score) =
                self.try_razor_prune(node, depth, bounds.alpha, ply, in_check, stack_eval)
            {
                return score;
            }

            if let Some(score) = self.try_null_move_prune(
                node,
                depth,
                ply,
                bounds,
                in_check,
                null_move_allowed,
                Some(corrected_eval),
                static_eval,
                tt_pv,
            ) {
                return score;
            }
        }

        let is_improving = !in_check && self.search_stack.is_improving();

        if let Some(score) = self.try_reverse_futility_prune(
            node,
            depth,
            in_check,
            corrected_eval,
            static_eval,
            bounds,
            ply,
            is_improving,
            tt_pv,
        ) {
            return score;
        }

        // Internal Iterative Reduction: reduce depth when no TT move is found.
        // https://www.chessprogramming.org/Internal_Iterative_Reductions
        let depth = if ply > 0 && tt_move.is_none() && depth >= self.config.iir_min_depth.value {
            depth.saturating_sub(self.config.iir_reduction.value)
        } else {
            depth
        };

        self.max_ply_reached = self.max_ply_reached.max(ply);

        let mut best_value = -SCORE_INF;
        let mut best_move = None;
        let mut best_move_depth = 0;

        let threats = node.threats();

        let best_move_hint = if ply == 0 {
            // At root we can use the currently best move for ordering
            self.multi_pv.best_move_hint()
        } else {
            tt_move
        };

        let enemy_attacks = node.attacks_for(!node.side_to_move());
        let mut movegen = MainMoveGenerator::new(
            best_move_hint,
            prev_moves,
            self.config.quiet_check_bonus.value,
            self.config.quiet_check_see_margin.value,
            self.config.bad_quiet_threshold.value,
            self.config.escape_divisor.value,
            self.config.unsafe_square_divisor.value,
            threats,
            enemy_attacks,
        );

        // Used for punishing potentially "bad" quiet moves that were searched before a potential beta cutoff
        let mut quiets_searched: ArrayVec<Move, { MAX_QUIETS }> = ArrayVec::new();
        let mut captures_searched: ArrayVec<Move, { MAX_CAPTURES }> = ArrayVec::new();

        let mut move_index = -1;
        while let Some(m) = movegen.next(
            node,
            &self.history_heuristic,
            &self.capture_history,
            &self.continuation_history,
        ) {
            if singular.is_some_and(|s| s.excluded == m) {
                continue;
            }

            // Let's not search the same move twice in different PVs
            if ply == 0 && self.multi_pv.is_excluded(m) {
                continue;
            }

            move_index += 1;

            if self.should_lmp_prune(node, m, in_check, depth, move_index, is_improving) {
                continue;
            }

            // Probe TT move for singular extension or multi-cut prune.
            let singular_result = self.probe_singular(
                node,
                m,
                tt_info,
                depth,
                ply,
                singular.is_some(),
                bounds.beta,
            );
            if let Some(value) = singular_result.multi_cut {
                return value;
            }
            if let Some((value, is_quiet, searched_depth)) = self.search_move(
                node,
                m,
                depth,
                ply,
                bounds,
                in_check,
                move_index,
                is_improving,
                tt_move_is_capture,
                corrected_eval,
                singular_result.extension,
                &prev_moves,
                tt_pv,
            ) {
                if self.shared.is_stopped() {
                    break;
                }

                if value > best_value {
                    best_value = value;
                    best_move = Some(m);
                    self.pv_table.update(ply, m);
                    best_move_depth = searched_depth;
                }

                bounds.raise_alpha(best_value);
                if bounds.is_cutoff(bounds.alpha) {
                    self.on_fail_high(
                        node,
                        m,
                        depth,
                        &quiets_searched,
                        &captures_searched,
                        &prev_moves,
                    );
                    break;
                }

                if is_quiet {
                    // If we have a quiet move later that causes a cutoff, then this
                    // move should have been sorted after, so let's punish it!
                    let _ = quiets_searched.try_push(m);
                } else {
                    // Similarly track captures that didn't cause cutoff
                    let _ = captures_searched.try_push(m);
                }
            }
        }

        // Check for terminal position (no legal moves)
        if move_index == -1 {
            return if in_check {
                -(MATE_VALUE - ply as i16) // Checkmate
            } else {
                0 // Stalemate
            };
        }

        // Let's not store a partial result in the TT / correction.
        if self.shared.is_stopped() {
            return best_value;
        }

        // Use original alpha when storing in tables, since the bound type depends on the original expectation.
        // Alpha may have been raised during search, but the bound type depends on
        // whether we improved.
        if singular.is_none() {
            self.shared.tt().store(
                hash,
                ply,
                depth,
                best_value,
                Some(static_eval),
                original_bounds.alpha,
                bounds.beta,
                best_move,
                tt_pv,
            );
        }

        self.shared.correction().update(
            node.board(),
            &prev_moves,
            in_check,
            best_move,
            best_value,
            corrected_eval,
            original_bounds.alpha,
            bounds.beta,
            best_move_depth,
        );

        best_value
    }

    /// Searches a single move.
    /// Returns None if pruned, otherwise (score, is_quiet, searched_depth).
    #[allow(clippy::too_many_arguments)]
    fn search_move(
        &mut self,
        node: &Node,
        m: Move,
        depth: u8,
        ply: u8,
        bounds: Bounds,
        in_check: bool,
        move_index: i32,
        is_improving: bool,
        tt_move_is_capture: bool,
        static_eval: i16,
        extra_extension: i8,
        prev_moves: &PrevMoves,
        tt_pv: bool,
    ) -> Option<(i16, bool, u8)> {
        let moved_color = node.board().side_to_move();
        let moved_piece = node.piece_on(m.from).unwrap();
        let is_cap = node.is_capture(m);
        let is_promotion = m.promotion == Some(Piece::Queen);
        let is_pv_node = node.is_pv();
        let is_pv_move = move_index == 0;

        if self.try_see_prune(
            node,
            m,
            moved_piece,
            is_cap,
            depth,
            in_check,
            is_pv_move,
            bounds.alpha,
            static_eval,
        ) {
            return None;
        }

        let hist = if is_cap {
            self.capture_history.get(node.board(), m)
        } else {
            self.history_heuristic.get(moved_color, m.from, m.to)
        };
        let cont_hist = {
            let pt = PieceTo::new(moved_color, moved_piece, m.to);
            self.continuation_history.get(prev_moves, pt)
        };

        if self.try_history_prune(depth, is_pv_move, is_cap, is_improving, hist, cont_hist) {
            return None;
        }

        let mut child = node.create_child(m, move_index);
        let child_hash = child.hash();

        self.shared.tt().prefetch(child_hash);

        let gives_check = child.in_check();
        let is_tactical = in_check || gives_check || is_cap || is_promotion;

        if self.try_futility_prune(
            depth,
            in_check,
            is_tactical,
            is_pv_move,
            bounds.alpha,
            static_eval,
        ) {
            return None;
        }

        let reduction = self.get_reduction(
            ply,
            depth,
            is_pv_move,
            is_improving,
            is_cap,
            is_promotion,
            move_index,
            node,
            &child,
            hist,
            cont_hist,
            tt_move_is_capture,
            tt_pv,
        );

        let extension = self.get_extension(node, &m, moved_piece, is_cap);
        let extension = (extension + extra_extension).max(0) as u8;

        let mut adjusted_depth = depth.saturating_add(extension).saturating_sub(reduction);

        let child_bounds = if is_pv_move {
            bounds.invert()
        } else {
            Bounds::null(bounds.alpha).invert()
        };

        self.search_stack
            .push_move(&child, m, moved_piece, moved_color);
        let mut value = -self.search_node(
            &child,
            adjusted_depth.saturating_sub(1),
            ply.saturating_add(1),
            child_bounds,
            true,
        );
        self.search_stack.pop();

        // Re-search without reduction if reduced search beat alpha
        if reduction > 0 && value > bounds.alpha {
            // Don't demote the PV move to a Cut node just because we reduced it.
            if is_pv_move && is_pv_node {
                child.set_type(NodeType::Pv);
            } else {
                child.set_type(child.node_type().inverted());
            }

            // Search at full depth
            adjusted_depth = depth.saturating_add(extension);

            self.search_stack
                .push_move(&child, m, moved_piece, moved_color);
            value = -self.search_node(
                &child,
                adjusted_depth.saturating_sub(1),
                ply.saturating_add(1),
                child_bounds,
                true,
            );
            self.search_stack.pop();
        }

        // Re-search with full window if null window failed high in a PV node
        if value > bounds.alpha && value < bounds.beta && !is_pv_move && is_pv_node {
            child.set_type(NodeType::Pv);

            self.search_stack
                .push_move(&child, m, moved_piece, moved_color);
            value = -self.search_node(
                &child,
                adjusted_depth.saturating_sub(1),
                ply.saturating_add(1),
                bounds.invert(),
                true,
            );
            self.search_stack.pop();
        }

        let is_quiet = !is_cap && !is_promotion;

        Some((value, is_quiet, adjusted_depth))
    }

    /// Updates the history tables after a beta cutoff: boost the cutting move,
    /// apply malus to the moves we searched before it.
    pub(super) fn on_fail_high(
        &mut self,
        node: &Node,
        mv: Move,
        depth: u8,
        quiets_searched: &[Move],
        captures_searched: &[Move],
        prev_moves: &PrevMoves,
    ) {
        let board = node.board();
        let is_quiet = !node.is_capture(mv);

        if is_quiet {
            let bonus = self.history_heuristic.get_bonus(depth);
            self.history_heuristic.update(board, mv, bonus);

            let cont_bonus = self.continuation_history.get_bonus(depth);
            self.continuation_history
                .update_quiet_all(board, prev_moves, mv, cont_bonus);
        } else {
            let bonus = self.capture_history.get_bonus(depth);
            self.capture_history.update_capture(board, mv, bonus);
        }

        if !quiets_searched.is_empty() {
            let quiet_malus = self.history_heuristic.get_malus(depth);
            let cont_malus = self.continuation_history.get_malus(depth);
            for &q in quiets_searched {
                self.history_heuristic.update(board, q, quiet_malus);
                self.continuation_history
                    .update_quiet_all(board, prev_moves, q, cont_malus);
            }
        }

        let capture_malus = self.capture_history.get_malus(depth);
        for &c in captures_searched {
            self.capture_history.update_capture(board, c, capture_malus);
        }
    }
}
