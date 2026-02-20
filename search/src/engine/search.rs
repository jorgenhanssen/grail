use std::sync::{Arc, atomic::Ordering, mpsc::Sender};

use arrayvec::ArrayVec;
use cozy_chess::{Move, Piece};
use evaluation::scores::{MATE_VALUE, SCORE_INF};
use uci::{
    UciOutput,
    commands::{GoParams, Info, Score},
};
use utils::{Node, NodeType, has_legal_moves};

use crate::{
    aspiration::Pass,
    engine::reduction::Reduction,
    move_ordering::{MAX_CAPTURES, MAX_QUIETS, MainMoveGenerator},
    pv::PvLine,
    result::SearchResult,
    stack::SearchNode,
    time_control::SearchController,
    transposition::{Bound, ProbeResult},
    utils::Bounds,
};

use super::{Engine, MAX_DEPTH, pruning::mate_distance_prune};

impl Engine {
    /// Multi-PV search with iterative deepening.
    ///
    /// Returns `None` if already in checkmate, otherwise returns all PV lines found.
    ///
    /// <https://www.chessprogramming.org/Iterative_Deepening>
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
        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || stop.store(true, Ordering::Relaxed));
        controller.start_timer();

        let pv_count = self.config.multi_pv.value as usize;
        let root = Node::new(self.board.clone(), NodeType::Pv);
        let mut depth = 1u8;

        // Iterative deepening
        while !self.stop.load(Ordering::Relaxed) && depth < MAX_DEPTH as u8 {
            controller.on_iteration_start();

            if !controller.should_continue_to_next_depth(depth) {
                break;
            }

            self.multi_pv.reset_excluded();

            for pv_index in 0..pv_count {
                if let Some(pv) = self.search_pv(&root, depth, pv_index, &mut controller) {
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

                if self.stop.load(Ordering::Relaxed) {
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

    /// Search for a single PV.
    fn search_pv(
        &mut self,
        root: &Node,
        depth: u8,
        pv_index: usize,
        controller: &mut SearchController,
    ) -> Option<PvLine> {
        self.multi_pv.begin_pv_search(pv_index, depth);

        let mut retries = 0;

        loop {
            let bounds = self.multi_pv.lines[pv_index].window.bounds();

            let (score, pv) = self.search_node(root, depth, 0, bounds, true);
            if pv.is_empty() {
                return None;
            }

            if self.stop.load(Ordering::Relaxed) {
                return Some(PvLine::new(pv, score, pv_index));
            }

            match self.multi_pv.lines[pv_index].window.analyse_pass(score) {
                Pass::Hit(s) => {
                    return Some(PvLine::new(pv, s, pv_index));
                }
                _ => {
                    controller.on_aspiration_failure();
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
        self.stop.store(false, Ordering::Relaxed);
        self.nodes = 0;
        self.max_ply_reached = 1;

        self.search_stack.clear();
        self.search_stack.push(SearchNode::new(self.board.hash()));

        self.tt.age();

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
    ) -> (i16, Vec<Move>) {
        let singular = self.search_stack.current().and_then(|n| n.singular);

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
        let original_bounds = bounds;
        let is_pv_node = node.is_pv();

        let tt_info: Option<ProbeResult> = if let Some(tt) = self.tt.probe(hash, ply) {
            // Only trust value/bound for cutoffs if the TT entry comes from a
            // search at least as deep as we need. Shallow results may have
            // missed tactics and can't safely prune the current search.
            //
            // Don't do TT cutoffs in PV nodes or during singular search.
            if !is_pv_node && singular.is_none() && tt.depth >= depth {
                match tt.bound {
                    // Exact: previous search found true minimax value
                    Bound::Exact => {
                        return (tt.value, tt.best_move.map_or(Vec::new(), |m| vec![m]));
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

            // Even if we are not able to return the TT move,
            // it is still valuable for cached static eval as hint for move ordering, etc.
            Some(tt)
        } else {
            None
        };

        let in_check = node.in_check();
        let tt_move = tt_info.and_then(|t| t.best_move);

        let static_eval = tt_info
            .and_then(|t| t.static_eval)
            .unwrap_or_else(|| self.static_eval(node));

        let corrected_eval = self.correction_history.adjust(node.board(), static_eval);

        self.search_stack
            .current_mut(|n| n.static_eval = Some(corrected_eval));

        // Stockfish skips NMP during singular searches.
        // Likely because NMP can raise beta without searching any actual moves,
        // so the "are all other moves worse?" test becomes unreliable.
        // I also saw success skipping razoring.
        if singular.is_none() {
            if let Some(score) =
                self.try_razor_prune(node, depth, bounds.alpha, ply, in_check, corrected_eval)
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
                Some(corrected_eval),
            ) {
                return (score, Vec::new());
            }
        }

        let is_improving = !in_check && self.search_stack.is_improving();

        if let Some(score) = self.try_reverse_futility_prune(
            node,
            depth,
            in_check,
            corrected_eval,
            bounds,
            ply,
            is_improving,
        ) {
            return (score, Vec::new());
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
        let mut best_line = Vec::new();

        let mut best_move_depth = 0;

        let threats = node.threats();

        let prev_moves = self
            .continuation_history
            .get_prev_moves(self.search_stack.as_slice());

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
                return (value, Vec::new());
            }
            if let Some((value, mut line, is_quiet, searched_depth)) = self.search_move(
                node,
                m,
                depth,
                ply,
                bounds,
                in_check,
                move_index,
                is_improving,
                corrected_eval,
                singular_result.extension,
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
                }

                bounds.raise_alpha(best_value);
                if bounds.is_cutoff(bounds.alpha) {
                    self.on_fail_high(
                        node,
                        m,
                        depth,
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

        // Use original alpha when storing in tables, since the bound type depends on the original expectation.
        // Alpha may have been raised during search, but the bound type depends on
        // whether we improved.

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

        self.correction_history.update(
            node.board(),
            in_check,
            best_move,
            best_value,
            corrected_eval,
            original_bounds.alpha,
            bounds.beta,
            best_move_depth,
        );

        (best_value, best_line)
    }

    /// Searches a single move with per-move pruning and LMR.
    /// Returns `None` if pruned, otherwise (score, pv, is_quiet, searched_depth).
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
        static_eval: i16,
        extra_extension: u8,
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
            is_improving,
            is_cap,
            is_promotion,
            move_index,
            node,
            &child,
            m,
            &self.lmr,
        ) {
            Reduction::Reduce(r) => r,
            Reduction::Prune => return None,
        };

        let mut extension = self.get_extension(node, &m, moved_piece, is_cap);
        extension = extension.saturating_add(extra_extension);

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

    /// Handler called if a search fails high - updates history tables.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_fail_high(
        &mut self,
        node: &Node,
        mv: Move,
        depth: u8,
        is_quiet: bool,
        quiets_searched: &[Move],
        captures_searched: &[Move],
    ) {
        let board = node.board();

        let prev_moves = self
            .continuation_history
            .get_prev_moves(self.search_stack.as_slice());
        if is_quiet {
            // Boost the quiet move that caused the cutoff
            let bonus = self.history_heuristic.get_bonus(depth);
            self.history_heuristic.update(board, mv, bonus);

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
                self.history_heuristic.update(board, q, quiet_malus);
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
