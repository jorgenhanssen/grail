use std::sync::{atomic::Ordering, mpsc::Sender, Arc};

use arrayvec::ArrayVec;
use cozy_chess::{BitBoard, Move, Piece};
use evaluation::scores::{MATE_VALUE, SCORE_INF};
use uci::{
    commands::{GoParams, Info, Score},
    UciOutput,
};
use utils::{flip_eval_perspective, has_legal_moves, Node, NodeType};

use crate::{
    engine::reduction::Reduction,
    move_ordering::{MainMoveGenerator, MAX_CAPTURES, MAX_QUIETS},
    pruning::{mate_distance_prune, should_lmp_prune, AspirationWindow, Pass},
    reductions::iir,
    stack::SearchNode,
    time_control::SearchController,
    transposition::Bound,
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
        );

        let mut controller =
            SearchController::new(params, &self.board, self.config.move_overhead.value as u64);
        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || stop.store(true, Ordering::Relaxed));
        controller.start_timer();

        let mut depth = 1;
        let mut best_move = None;
        let mut best_score = 0;

        // Root node is always PV
        let root = Node::new(self.board.clone(), NodeType::Pv);

        while !self.stop.load(Ordering::Relaxed) && depth <= MAX_DEPTH as u8 {
            controller.on_iteration_start();

            if !controller.should_continue_to_next_depth(depth) {
                break;
            }

            window.begin_depth(depth, best_score);
            let mut retries = 0;

            loop {
                let (alpha, beta) = window.bounds();
                let (mv, score) = self.search_root(&root, depth, alpha, beta);

                if mv.is_none() {
                    break;
                }

                match window.analyse_pass(score) {
                    Pass::Hit(s) => {
                        best_move = mv;
                        best_score = s;

                        controller.on_iteration_complete(depth, s, mv);

                        if let Some(out) = output {
                            self.send_search_info(out, depth, s, controller.elapsed());
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

            depth += 1;
        }

        best_move.map(|mv| (mv, best_score))
    }

    /// Initializes the search - resets all state for a new search.
    fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);

        self.nodes = 0;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        self.search_stack.clear();
        self.search_stack.push(SearchNode::new(self.board.hash()));

        self.tt.age();
    }

    /// Root search with the given alpha-beta window.
    /// Called once per aspiration window attempt at each depth.
    ///
    /// TODO: This function duplicates much of search_subtree() and search_move() (PVS windowing,
    /// LMR, re-search logic, PV creation). Consider unifying into a single search_node() function
    /// to simplify.
    pub(super) fn search_root(
        &mut self,
        root: &Node,
        depth: u8,
        mut alpha: i16,
        beta: i16,
    ) -> (Option<Move>, i16) {
        let best_move = self.current_pv.first().cloned();
        let threats = root.threats();

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(self.search_stack.as_slice());
        let mut moves = MainMoveGenerator::new(
            best_move,
            [None; 2],
            prev_to,
            self.config.quiet_check_bonus.value,
            threats,
        );

        let mut best_score = -SCORE_INF;
        let mut current_best_move = None;

        let in_check = root.in_check();
        let remaining_depth = depth.saturating_sub(1);
        let mut move_index: i32 = -1;
        while let Some(m) = moves.next(
            root,
            &self.history_heuristic,
            &self.capture_history,
            &self.continuation_history,
        ) {
            move_index += 1;
            let is_pv_move = move_index == 0;

            let moved_piece = root.piece_on(m.from).unwrap();
            let mut child = root.create_child(m, move_index);

            self.search_stack.push_move(&child, m, moved_piece);

            // LMR: reduce late non-tactical moves
            let gives_check = child.in_check();
            let is_cap = root.is_capture(m);
            let is_promotion = m.promotion.is_some();
            let is_tactical = in_check || gives_check || is_cap || is_promotion;

            let reduction = match self.get_reduction(
                depth,
                remaining_depth,
                is_pv_move,
                is_tactical,
                true,
                is_cap,
                move_index,
                root,
                m,
                depth,
                root.threats(),
                child.threats(),
                &self.lmr,
            ) {
                Reduction::Reduction(r) => r,
                Reduction::Prune => continue,
            };

            // PVS window
            let alpha_child = alpha;
            let beta_child = if is_pv_move {
                beta
            } else {
                alpha_child.saturating_add(1)
            };

            // Initial search (possibly reduced depth, null window for non-PV moves)
            let reduced_depth = depth.saturating_sub(reduction);
            let (child_value, mut pv) =
                self.search_subtree(&child, 1, reduced_depth, -beta_child, -alpha_child, true);
            let mut score = -child_value;

            // LMR re-search: reduced search beat alpha, verify at full depth
            if reduction > 0 && score > alpha_child {
                child.set_type(child.node_type().inverted());
                let (re_child_value, re_pv) =
                    self.search_subtree(&child, 1, depth, -beta_child, -alpha_child, true);
                score = -re_child_value;
                pv = re_pv;
            }

            // PVS re-search: null window beat alpha, verify with full window
            if !is_pv_move && score > alpha_child && score < beta {
                child.set_type(NodeType::Pv);
                let (full_child_value, full_pv) =
                    self.search_subtree(&child, 1, depth, -beta, -alpha_child, true);
                score = -full_child_value;
                pv = full_pv;
            }
            self.search_stack.pop();

            // Check if we were stopped during the subtree search
            if self.stop.load(Ordering::Relaxed) {
                return (None, 0);
            }

            pv.insert(0, m);

            if score > best_score {
                best_score = score;
                current_best_move = Some(m);
                self.current_pv = pv;
            }

            alpha = alpha.max(best_score);

            // Beta cutoff
            if alpha >= beta {
                break;
            }
        }

        (current_best_move, best_score)
    }

    /// Recursive alpha-beta search with PVS.
    ///
    /// Applies pruning, reductions, and searches child nodes.
    /// Returns (score, pv).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn search_subtree(
        &mut self,
        node: &Node,
        depth: u8,
        max_depth: u8,
        mut alpha: i16,
        mut beta: i16,
        null_move_allowed: bool,
    ) -> (i16, Vec<Move>) {
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }
        self.nodes += 1;

        // If this position has been seen before, treat it as a draw
        if self.search_stack.is_repetition(&self.game_history) {
            return (0, Vec::new());
        }

        let hash = node.hash();
        if mate_distance_prune(&mut alpha, &mut beta, depth) {
            return (alpha, Vec::new());
        }

        if depth >= max_depth {
            return self.quiescence_search(node, alpha, beta, depth);
        }

        // Transposition table probe
        let original_alpha = alpha;
        let mut maybe_tt_move = None;
        let mut tt_static_eval = None;

        let is_pv_node = node.is_pv();

        if let Some(tt) = self.tt.probe(hash, depth) {
            // Only trust value/bound for cutoffs if the TT entry comes from a
            // search at least as deep as we need. Shallow results may have
            // missed tactics and can't safely prune the current search.
            //
            // Don't do TT cutoffs in PV nodes - TT only stores one move,
            // so cutting off here could truncate the PV to just that move.
            let needed_depth = max_depth - depth;
            if !is_pv_node && tt.depth >= needed_depth {
                match tt.bound {
                    // Exact: previous search found true minimax value
                    Bound::Exact => {
                        return (tt.value, tt.best_move.map_or(Vec::new(), |m| vec![m]))
                    }
                    // Lower: previous search failed high (value >= beta), so value is at least this good
                    Bound::Lower => {
                        alpha = alpha.max(tt.value);
                        if alpha >= beta {
                            return (tt.value, tt.best_move.map_or(Vec::new(), |m| vec![m]));
                        }
                    }
                    // Upper: previous search failed low (value <= alpha), so value is at most this bad
                    Bound::Upper => {
                        beta = beta.min(tt.value);
                        if beta <= alpha {
                            return (beta, tt.best_move.map_or(Vec::new(), |m| vec![m]));
                        }
                    }
                }
            }

            // However, we can use the TT move for ordering and static eval for caching,
            // even from shallow searches - these are still valuable hints!
            maybe_tt_move = tt.best_move;
            tt_static_eval = tt.static_eval;
        }

        let phase = node.game_phase();
        let in_check = node.in_check();
        let remaining_depth = max_depth - depth;

        // Internal Iterative Reductions
        let (max_depth, remaining_depth) = iir(
            max_depth,
            remaining_depth,
            maybe_tt_move.is_some(),
            self.config.iir_min_depth.value,
            self.config.iir_reduction.value,
        );

        let static_eval = if let Some(tt_se) = tt_static_eval {
            tt_se // Cached in TT
        } else {
            let eval = self.eval(node, phase);
            flip_eval_perspective(node.side_to_move(), eval)
        };

        self.search_stack
            .current_mut(|n| n.static_eval = Some(static_eval));

        if let Some(score) =
            self.try_razor_prune(node, remaining_depth, alpha, depth, in_check, static_eval)
        {
            return (score, Vec::new());
        }

        if let Some(score) = self.try_null_move_prune(
            node,
            depth,
            max_depth,
            alpha,
            beta,
            remaining_depth,
            in_check,
            null_move_allowed,
            Some(static_eval),
        ) {
            return (score, Vec::new());
        }

        let is_improving = !in_check && self.search_stack.is_improving();

        if let Some(score) = self.try_reverse_futility_prune(
            node,
            remaining_depth,
            in_check,
            static_eval,
            beta,
            depth,
            alpha,
            is_improving,
        ) {
            return (score, Vec::new());
        }

        self.max_depth_reached = self.max_depth_reached.max(depth);

        let mut best_value = -SCORE_INF;
        let mut best_move = None;
        let mut best_line = Vec::new();

        let mut best_move_depth = depth;

        let threats = node.threats();

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(self.search_stack.as_slice());

        let mut movegen = MainMoveGenerator::new(
            maybe_tt_move,
            self.killer_moves[depth as usize],
            prev_to,
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

            // Late Move Pruning (LMP)
            if should_lmp_prune(
                node,
                m,
                in_check,
                remaining_depth,
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
                max_depth,
                alpha,
                beta,
                in_check,
                remaining_depth,
                m,
                move_index,
                is_improving,
                static_eval,
                threats,
            ) {
                if self.stop.load(Ordering::Relaxed) {
                    break;
                }

                if value > best_value {
                    best_value = value;
                    best_move = Some(m);
                    line.insert(0, m);
                    best_line = line;
                    best_move_depth = searched_depth;
                }

                alpha = alpha.max(best_value);
                if alpha >= beta {
                    self.on_fail_high(
                        node,
                        m,
                        remaining_depth,
                        depth as usize,
                        is_quiet,
                        &quiets_searched,
                        &captures_searched,
                    );

                    break; // beta cutoff
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
                (-(MATE_VALUE - depth as i16), Vec::new()) // Checkmate
            } else {
                (0, Vec::new()) // Stalemate
            };
        }

        // Store TT entry with the depth actually searched for the best move
        self.tt.store(
            hash,
            depth,
            best_move_depth,
            best_value,
            Some(static_eval),
            original_alpha,
            beta,
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
        max_depth: u8,
        alpha: i16,
        beta: i16,
        in_check: bool,
        remaining_depth: u8,
        m: Move,
        move_index: i32,
        is_improving: bool,
        static_eval: i16,
        pre_move_threats: BitBoard,
    ) -> Option<(i16, Vec<Move>, bool, u8)> {
        let moved_piece = node.piece_on(m.from).unwrap();
        let mut child = node.create_child(m, move_index);
        let child_hash = child.hash();

        self.tt.prefetch(child_hash);

        let gives_check = child.in_check();

        // Consider move tactical if it's check, capture, or promotion
        let is_cap = node.is_capture(m);
        let is_promotion = m.promotion == Some(Piece::Queen);
        let is_tactical = in_check || gives_check || is_cap || is_promotion;
        let is_pv_node = node.is_pv();
        let is_pv_move = move_index == 0;

        if self.try_futility_prune(remaining_depth, in_check, is_tactical, alpha, static_eval) {
            return None;
        }

        if self.try_see_prune(
            node,
            m,
            moved_piece,
            remaining_depth,
            in_check,
            is_pv_move,
            alpha,
            static_eval,
        ) {
            return None;
        }

        let reduction = match self.get_reduction(
            depth,
            remaining_depth,
            is_pv_move,
            is_tactical,
            is_improving,
            is_cap,
            move_index,
            node,
            m,
            max_depth,
            pre_move_threats,
            child.threats(),
            &self.lmr,
        ) {
            Reduction::Reduction(r) => r,
            Reduction::Prune => return None,
        };

        let extension = self.get_extension(node, &m, moved_piece, is_cap);

        let extended_max_depth = max_depth + extension;
        let reduced_max_depth = extended_max_depth.saturating_sub(reduction).max(depth + 1);
        let mut searched_depth = reduced_max_depth;

        let alpha_child = alpha;
        let beta_child = if is_pv_move { beta } else { alpha + 1 };

        // Initial search (reduced if LMR, null window if not first move)
        self.search_stack.push_move(&child, m, moved_piece);
        let (child_value, pv_line) = self.search_subtree(
            &child,
            depth + 1,
            reduced_max_depth,
            -beta_child,
            -alpha_child,
            true,
        );
        self.search_stack.pop();
        let mut value = -child_value;
        let mut line = pv_line;

        // Re-search at full depth (if LMR was used and value > alpha)
        if reduction > 0 && value > alpha {
            child.set_type(child.node_type().inverted());
            self.search_stack.push_move(&child, m, moved_piece);
            let (re_child_value, re_line) = self.search_subtree(
                &child,
                depth + 1,
                extended_max_depth,
                -beta_child,
                -alpha_child,
                true,
            );
            self.search_stack.pop();
            value = -re_child_value;
            line = re_line;
            searched_depth = extended_max_depth;
        }

        // Re-search with full window (if null window failed high in a PV node)
        if value > alpha && value < beta && !is_pv_move && is_pv_node {
            child.set_type(NodeType::Pv);
            self.search_stack.push_move(&child, m, moved_piece);
            let (full_child_value, full_line) =
                self.search_subtree(&child, depth + 1, extended_max_depth, -beta, -alpha, true);
            self.search_stack.pop();
            value = -full_child_value;
            line = full_line;
            searched_depth = extended_max_depth;
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
        remaining_depth: u8,
        depth: usize,
        is_quiet: bool,
        quiets_searched: &[Move],
        captures_searched: &[Move],
    ) {
        let board = node.board();
        let threats = node.threats();

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(self.search_stack.as_slice());
        if is_quiet {
            // Add killer move for quiet moves
            let killers = &mut self.killer_moves[depth];
            if killers[0] != Some(mv) {
                killers[1] = killers[0];
                killers[0] = Some(mv);
            }

            // Boost the quiet move that caused the cutoff
            let bonus = self.history_heuristic.get_bonus(remaining_depth);
            self.history_heuristic.update(board, mv, bonus, threats);

            // Continuation history bonus for quiet cutoff move
            let cont_bonus = self.continuation_history.get_bonus(remaining_depth);
            self.continuation_history
                .update_quiet_all(board, &prev_to, mv, cont_bonus);
        } else {
            // Boost the capture that caused the cutoff
            let bonus = self.capture_history.get_bonus(remaining_depth);
            self.capture_history.update_capture(board, mv, bonus);
        }

        if !quiets_searched.is_empty() {
            // Apply malus to all previously searched quiet moves
            let quiet_malus = self.history_heuristic.get_malus(remaining_depth);
            for &q in quiets_searched {
                self.history_heuristic
                    .update(board, q, quiet_malus, threats);
            }

            // Continuation history malus for previously searched quiets
            let cont_malus = self.continuation_history.get_malus(remaining_depth);
            for &q in quiets_searched {
                self.continuation_history
                    .update_quiet_all(board, &prev_to, q, cont_malus);
            }
        }

        if !captures_searched.is_empty() {
            // Apply malus to all previously searched captures
            let capture_malus = self.capture_history.get_malus(remaining_depth);
            for &c in captures_searched {
                self.capture_history.update_capture(board, c, capture_malus);
            }
        }
    }
}
