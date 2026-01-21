use std::sync::{atomic::Ordering, mpsc::Sender, Arc};

use arrayvec::ArrayVec;
use cozy_chess::{Move, Piece};
use evaluation::scores::{MATE_VALUE, SCORE_INF};
use uci::{
    commands::{GoParams, Info, Score},
    UciOutput,
};
use utils::{has_legal_moves, Node, NodeType};

use crate::{
    engine::reduction::Reduction,
    move_ordering::{MainMoveGenerator, MAX_CAPTURES, MAX_QUIETS},
    pruning::{mate_distance_prune, should_lmp_prune, AspirationWindow, Pass},
    reductions::iir,
    stack::SearchNode,
    time_control::SearchController,
    transposition::Bound,
    utils::Bounds,
};

use super::{Engine, MAX_DEPTH};

impl Engine {
    /// Iterative deepening search with aspiration windows.
    ///
    /// Returns the best move and score, or `None` if already in checkmate.
    pub fn search(
        &mut self,
        params: &GoParams,
        output: Option<&Sender<UciOutput>>,
    ) -> Option<(Move, i16)> {
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

        let mut window = AspirationWindow::new(
            self.config.aspiration_window_size.value,
            self.config.aspiration_window_widen.value,
            self.config.aspiration_window_depth.value,
            self.config.aspiration_score_divisor.value,
        );

        let mut controller =
            SearchController::new(params, &self.board, self.config.move_overhead.value as u64);
        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || stop.store(true, Ordering::Relaxed));
        controller.start_timer();

        let mut iteration = 1;
        let mut best_move = None;
        let mut best_score = 0;

        // Root node is always PV
        let root = Node::new(self.board.clone(), NodeType::Pv);

        while !self.stop.load(Ordering::Relaxed) && iteration < MAX_DEPTH as u8 {
            controller.on_iteration_start();

            if !controller.should_continue_to_next_depth(iteration) {
                break;
            }

            window.begin_depth(iteration, best_score);
            let mut retries = 0;

            loop {
                let bounds = window.bounds();
                let (score, pv) = self.search_node(&root, iteration, 0, bounds, true);
                let mv = pv.first().cloned();

                if mv.is_none() {
                    break;
                }

                // If stopped during search, use partial results from current iteration
                // This is often better than using the known best from the previous iteration
                if self.stop.load(Ordering::Relaxed) {
                    best_move = mv;
                    best_score = score;
                    break;
                }

                match window.analyse_pass(score) {
                    Pass::Hit(s) => {
                        best_move = mv;
                        best_score = s;

                        controller.on_iteration_complete(iteration, s, mv);

                        if let Some(out) = output {
                            self.send_search_info(out, iteration, s, controller.elapsed());
                        }
                        break;
                    }
                    _ => {
                        controller.on_aspiration_failure();

                        retries += 1;

                        if retries >= self.config.aspiration_window_retries.value {
                            window.fully_extend();
                            retries = 0;
                        }
                    }
                }
            }

            iteration += 1;
        }

        best_move.map(|mv| (mv, best_score))
    }

    /// Initializes the search - resets all state for a new search.
    fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);

        self.nodes = 0;
        self.max_ply_reached = 1;
        self.current_pv.clear();

        self.search_stack.clear();
        self.search_stack.push(SearchNode::new(self.board.hash()));

        self.tt.age();
    }

    /// Alpha-beta search with principal variation search (PVS) and late move reductions.
    pub(super) fn search_node(
        &mut self,
        node: &Node,
        depth: u8,
        ply: u8,
        mut bounds: Bounds,
        null_move_allowed: bool,
    ) -> (i16, Vec<Move>) {
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }
        self.nodes += 1;

        if ply > 0 && self.is_forced_draw(node) {
            return (self.draw_value(), Vec::new());
        }

        // As deep as we can go, so return static eval
        if ply as usize >= MAX_DEPTH {
            return (self.static_eval(node), Vec::new());
        }

        if mate_distance_prune(&mut bounds, ply) {
            return (bounds.alpha, Vec::new());
        }

        if depth == 0 {
            return self.quiescence_search(node, bounds, ply);
        }

        let hash = node.hash();

        // Transposition table probe
        let original_bounds = bounds;
        let mut maybe_tt_move = None;
        let mut tt_static_eval = None;

        let is_pv_node = node.is_pv();

        if let Some(tt) = self.tt.probe(hash, ply) {
            // Only trust value/bound for cutoffs if the TT entry comes from a
            // search at least as deep as we need. Shallow results may have
            // missed tactics and can't safely prune the current search.
            //
            // Don't do TT cutoffs in PV nodes.
            if !is_pv_node && tt.depth >= depth {
                match tt.bound {
                    // Exact: previous search found true minimax value
                    Bound::Exact => {
                        return (tt.value, tt.best_move.map_or(Vec::new(), |m| vec![m]))
                    }
                    // Lower: previous search failed high (value >= beta), so value is at least this good
                    Bound::Lower => {
                        bounds.raise_alpha(tt.value);
                        if bounds.is_cutoff(bounds.alpha) {
                            return (tt.value, tt.best_move.map_or(Vec::new(), |m| vec![m]));
                        }
                    }
                    // Upper: previous search failed low (value <= alpha), so value is at most this bad
                    Bound::Upper => {
                        bounds.beta = bounds.beta.min(tt.value);
                        if bounds.beta <= bounds.alpha {
                            return (bounds.beta, tt.best_move.map_or(Vec::new(), |m| vec![m]));
                        }
                    }
                }
            }

            // However, we can use the TT move for ordering and static eval for caching,
            // even from shallow searches - these are still valuable hints!
            maybe_tt_move = tt.best_move;
            tt_static_eval = tt.static_eval;
        }

        let in_check = node.in_check();

        let depth = iir(
            ply,
            depth,
            maybe_tt_move.is_some(),
            self.config.iir_min_depth.value,
            self.config.iir_reduction.value,
        );

        let static_eval = tt_static_eval.unwrap_or_else(|| self.static_eval(node));

        self.search_stack
            .current_mut(|n| n.static_eval = Some(static_eval));

        if let Some(score) =
            self.try_razor_prune(node, depth, bounds.alpha, ply, in_check, static_eval)
        {
            return (score, Vec::new());
        }

        if let Some(score) = self.try_null_move_prune(
            node,
            depth,
            ply,
            bounds,
            in_check,
            null_move_allowed,
            Some(static_eval),
        ) {
            return (score, Vec::new());
        }

        let is_improving = !in_check && self.search_stack.is_improving();

        if let Some(score) = self.try_reverse_futility_prune(
            node,
            depth,
            in_check,
            static_eval,
            bounds,
            ply,
            is_improving,
        ) {
            return (score, Vec::new());
        }

        self.max_ply_reached = self.max_ply_reached.max(ply);

        let mut best_value = -SCORE_INF;
        let mut best_move = None;
        let mut best_line = Vec::new();

        let mut best_move_depth = 0;

        let threats = node.threats();

        let prev_moves = self
            .continuation_history
            .get_prev_moves(self.search_stack.as_slice());

        let best_move_hint = if ply == 0 {
            // At root we can use the currently best move (pv[0]) for ordering
            self.current_pv.first().cloned()
        } else {
            maybe_tt_move
        };
        let killers = self.killer_moves[ply as usize];

        let mut movegen = MainMoveGenerator::new(
            best_move_hint,
            killers,
            prev_moves,
            self.config.quiet_check_bonus.value,
            threats,
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
            move_index += 1;

            if should_lmp_prune(
                node,
                m,
                in_check,
                depth,
                move_index,
                is_improving,
                self.config.lmp_max_depth.value,
                self.config.lmp_base_moves.value,
                self.config.lmp_depth_multiplier.value,
                self.config.lmp_improving_reduction.value,
            ) {
                continue;
            }

            if let Some((value, mut line, is_quiet, searched_depth)) = self.search_move(
                node,
                depth,
                ply,
                bounds,
                in_check,
                m,
                move_index,
                is_improving,
                static_eval,
            ) {
                if self.stop.load(Ordering::Relaxed) {
                    break;
                }

                if value > best_value {
                    best_value = value;
                    best_move = Some(m);
                    line.insert(0, m);
                    best_line = line.clone();
                    best_move_depth = searched_depth;

                    // If we are at root and this is a great move,
                    // then let's use this move as the current PV.
                    if ply == 0 {
                        self.current_pv = line;
                    }
                }

                bounds.raise_alpha(best_value);
                if bounds.is_cutoff(bounds.alpha) {
                    self.on_fail_high(
                        node,
                        m,
                        depth,
                        ply as usize,
                        is_quiet,
                        &quiets_searched,
                        &captures_searched,
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
            // No moves were found - either checkmate or stalemate
            return if in_check {
                (-(MATE_VALUE - ply as i16), Vec::new()) // Checkmate
            } else {
                (0, Vec::new()) // Stalemate
            };
        }

        self.tt.store(
            hash,
            ply,
            best_move_depth,
            best_value,
            Some(static_eval),
            original_bounds.alpha,
            bounds.beta,
            best_move,
        );
        (best_value, best_line)
    }

    /// Searches a single move with per-move pruning and LMR.
    /// Returns `None` if the move was pruned, otherwise (score, pv, is_quiet, depth).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn search_move(
        &mut self,
        node: &Node,
        depth: u8,
        ply: u8,
        bounds: Bounds,
        in_check: bool,
        m: Move,
        move_index: i32,
        is_improving: bool,
        static_eval: i16,
    ) -> Option<(i16, Vec<Move>, bool, u8)> {
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

        let mut child = node.create_child(m, move_index);
        let child_hash = child.hash();

        self.tt.prefetch(child_hash);

        let gives_check = child.in_check();
        let is_tactical = in_check || gives_check || is_cap || is_promotion;

        if self.try_futility_prune(depth, in_check, is_tactical, bounds.alpha, static_eval) {
            return None;
        }

        let reduction = match self.get_reduction(
            ply,
            depth,
            is_pv_move,
            is_tactical,
            is_improving,
            is_cap,
            move_index,
            node,
            &child,
            m,
            &self.lmr,
        ) {
            Reduction::Reduction(r) => r,
            Reduction::Prune => return None,
        };

        let extension = self.get_extension(node, &m, moved_piece, is_cap);

        // Child's remaining depth after extension/reduction
        let extended_child_depth = depth.saturating_sub(1).saturating_add(extension);
        let reduced_child_depth = extended_child_depth.saturating_sub(reduction);
        // Effective depth we searched
        let mut searched_depth = depth
            .saturating_add(extension)
            .saturating_sub(reduction)
            .max(1);

        // Build child bounds: full window for PV move, null window otherwise
        let child_bounds = if is_pv_move {
            bounds.invert()
        } else {
            Bounds::null(bounds.alpha).invert()
        };

        // Initial search (reduced if LMR, null window if not first move)
        self.search_stack
            .push_move(&child, m, moved_piece, moved_color);
        let (child_value, pv_line) =
            self.search_node(&child, reduced_child_depth, ply + 1, child_bounds, true);
        self.search_stack.pop();
        let mut value = -child_value;
        let mut line = pv_line;

        // Re-search at full depth (if LMR was used and value > alpha)
        if reduction > 0 && value > bounds.alpha {
            child.set_type(child.node_type().inverted());
            self.search_stack
                .push_move(&child, m, moved_piece, moved_color);
            let (re_child_value, re_line) =
                self.search_node(&child, extended_child_depth, ply + 1, child_bounds, true);
            self.search_stack.pop();
            value = -re_child_value;
            line = re_line;
            searched_depth = depth.saturating_add(extension).max(1);
        }

        // Re-search with full window (if null window failed high in a PV node)
        if value > bounds.alpha && value < bounds.beta && !is_pv_move && is_pv_node {
            child.set_type(NodeType::Pv);
            self.search_stack
                .push_move(&child, m, moved_piece, moved_color);
            let (full_child_value, full_line) =
                self.search_node(&child, extended_child_depth, ply + 1, bounds.invert(), true);
            self.search_stack.pop();
            value = -full_child_value;
            line = full_line;
            searched_depth = depth.saturating_add(extension).max(1);
        }

        let is_quiet = !is_cap && !is_promotion;
        Some((value, line, is_quiet, searched_depth))
    }

    /// Handler called if a search fails high - updates history tables, killers, etc.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_fail_high(
        &mut self,
        node: &Node,
        mv: Move,
        depth: u8,
        ply: usize,
        is_quiet: bool,
        quiets_searched: &[Move],
        captures_searched: &[Move],
    ) {
        let board = node.board();
        let threats = node.threats();

        let prev_moves = self
            .continuation_history
            .get_prev_moves(self.search_stack.as_slice());
        if is_quiet {
            // Add killer move for quiet moves
            let killers = &mut self.killer_moves[ply];
            if killers[0] != Some(mv) {
                killers[1] = killers[0];
                killers[0] = Some(mv);
            }

            // Boost the quiet move that caused the cutoff
            let bonus = self.history_heuristic.get_bonus(depth);
            self.history_heuristic.update(board, mv, bonus, threats);

            // Continuation history bonus for quiet cutoff move
            let cont_bonus = self.continuation_history.get_bonus(depth);
            self.continuation_history
                .update_quiet_all(board, &prev_moves, mv, cont_bonus);
        } else {
            // Boost the capture that caused the cutoff
            let bonus = self.capture_history.get_bonus(depth);
            self.capture_history.update_capture(board, mv, bonus);
        }

        if !quiets_searched.is_empty() {
            // Apply malus to all previously searched quiet moves
            let quiet_malus = self.history_heuristic.get_malus(depth);
            for &q in quiets_searched {
                self.history_heuristic
                    .update(board, q, quiet_malus, threats);
            }

            // Continuation history malus for previously searched quiets
            let cont_malus = self.continuation_history.get_malus(depth);
            for &q in quiets_searched {
                self.continuation_history
                    .update_quiet_all(board, &prev_moves, q, cont_malus);
            }
        }

        if !captures_searched.is_empty() {
            // Apply malus to all previously searched captures
            let capture_malus = self.capture_history.get_malus(depth);
            for &c in captures_searched {
                self.capture_history.update_capture(board, c, capture_malus);
            }
        }
    }
}
