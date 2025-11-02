use std::sync::{atomic::Ordering, mpsc::Sender, Arc};

use arrayvec::ArrayVec;
use chess::{BitBoard, Board, BoardStatus, ChessMove, Color, Piece};
use evaluation::scores::{MATE_VALUE, NEG_INFINITY};
use uci::{
    commands::{GoParams, Info, Score},
    UciOutput,
};
use utils::{game_phase, Position};

use crate::{
    move_ordering::{MainMoveGenerator, MAX_CAPTURES, MAX_QUIETS},
    pruning::{lmr, mate_distance_prune, should_lmp_prune, AspirationWindow, Pass},
    stack::SearchNode,
    time_control::SearchController,
    transposition::Bound,
};

use super::{Engine, MAX_DEPTH};

impl Engine {
    pub fn search(
        &mut self,
        params: &GoParams,
        output: Option<&Sender<UciOutput>>,
    ) -> Option<(ChessMove, i16)> {
        if self.board.status() == BoardStatus::Checkmate {
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

        let mut controller = SearchController::new(params, &self.board);
        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || stop.store(true, Ordering::Relaxed));
        controller.start_timer();

        let mut depth = 1;
        let mut best_move = None;
        let mut best_score = 0;

        while !self.stop.load(Ordering::Relaxed) && depth <= MAX_DEPTH as u8 {
            controller.on_iteration_start();

            if !controller.should_continue_to_next_depth(depth) {
                break;
            }

            window.begin_depth(depth, best_score);
            let mut retries = 0;

            loop {
                let (alpha, beta) = window.bounds();
                let (mv, score) = self.search_root(depth, alpha, beta);

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

    #[inline(always)]
    fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);

        self.nodes = 0;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        self.search_stack.clear();
        self.search_stack
            .push(SearchNode::new(self.board.get_hash()));

        self.tt.age();
    }

    pub(super) fn search_root(
        &mut self,
        depth: u8,
        mut alpha: i16,
        beta: i16,
    ) -> (Option<ChessMove>, i16) {
        let best_move = self.current_pv.first().cloned();

        let position = Position::new(&self.board);
        let threats = position.threats_for(self.board.side_to_move());

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(self.search_stack.as_slice());
        let mut moves = MainMoveGenerator::new(
            best_move,
            [None; 2],
            &prev_to,
            game_phase(&self.board),
            self.config.get_piece_values(),
            self.config.quiet_check_bonus.value,
            threats,
        );

        let mut best_score = NEG_INFINITY;
        let mut current_best_move = None;

        // Negamax at root: call search_subtree with flipped window, then negate result
        while let Some(m) = moves.next(
            &self.board,
            &self.history_heuristic,
            &self.capture_history,
            &self.continuation_history,
        ) {
            let moved_piece = self.board.piece_on(m.get_source()).unwrap();
            let new_board = self.board.make_move_new(m);

            self.search_stack
                .push_move(new_board.get_hash(), m, moved_piece);
            let (child_value, mut pv) =
                self.search_subtree(&new_board, 1, depth, -beta, -alpha, true, true);
            let score = -child_value;
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
        }

        (current_best_move, best_score)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn search_subtree(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        mut alpha: i16,
        mut beta: i16,
        try_null_move: bool,
        allow_iid: bool,
    ) -> (i16, Vec<ChessMove>) {
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }
        self.nodes += 1;

        if self.search_stack.has_duplicate() {
            return (0, Vec::new()); // repetition = draw
        }

        let hash = self.search_stack.current().hash;
        if mate_distance_prune(&mut alpha, &mut beta, depth) {
            return (alpha, Vec::new());
        }

        if depth >= max_depth {
            return self.quiescence_search(board, alpha, beta, depth);
        }

        // Transposition table probe
        let original_alpha = alpha;
        let mut maybe_tt_move = None;
        let mut tt_static_eval = None;

        let is_pv_node = beta > alpha + 1;

        if let Some((tt_value, tt_bound, tt_move, tt_se)) = self.tt.probe(hash, depth, max_depth) {
            maybe_tt_move = tt_move;
            tt_static_eval = tt_se;
            match tt_bound {
                Bound::Exact => return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m])),
                Bound::Lower => {
                    alpha = alpha.max(tt_value);
                    if alpha >= beta {
                        return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m]));
                    }
                }
                Bound::Upper => {
                    if tt_value <= alpha {
                        return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m]));
                    }
                }
            }
        } else if let Some((tt_move, tt_se)) = self.tt.probe_hint(hash) {
            // Use shallow entry as hint for move ordering and static eval caching
            maybe_tt_move = tt_move;
            tt_static_eval = tt_se;
        }

        let phase = game_phase(board);
        let remaining_depth = max_depth - depth;
        let in_check = board.checkers().popcnt() > 0;

        let position = Position::new(board);

        let static_eval = if let Some(tt_se) = tt_static_eval {
            tt_se // Cached in TT
        } else {
            let eval = self.eval(&position, phase);
            if board.side_to_move() == Color::White {
                eval
            } else {
                -eval
            }
        };

        self.search_stack
            .current_mut(|node| node.static_eval = Some(static_eval));

        if let Some(score) =
            self.try_razor_prune(board, remaining_depth, alpha, depth, in_check, static_eval)
        {
            return (score, Vec::new());
        }

        if let Some(score) = self.try_null_move_prune(
            board,
            depth,
            max_depth,
            alpha,
            beta,
            hash,
            remaining_depth,
            in_check,
            try_null_move,
            Some(static_eval),
        ) {
            return (score, Vec::new());
        }

        // Internal Iterative Deepening (IID)
        if let Some(m) = self.try_iid(
            board,
            depth,
            max_depth,
            alpha,
            beta,
            try_null_move,
            allow_iid,
            maybe_tt_move.is_none(),
            remaining_depth,
            in_check,
        ) {
            maybe_tt_move = Some(m);
        }

        let is_improving = !in_check && self.search_stack.is_improving();

        if let Some(score) = self.try_reverse_futility_prune(
            remaining_depth,
            in_check,
            is_pv_node,
            static_eval,
            beta,
            hash,
            depth,
            max_depth,
            alpha,
            is_improving,
        ) {
            return (score, Vec::new());
        }

        self.max_depth_reached = self.max_depth_reached.max(depth);

        let mut best_value = NEG_INFINITY;
        let mut best_move = None;
        let mut best_line = Vec::new();

        let mut best_move_depth = depth;

        let threats = position.threats_for(board.side_to_move());

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(self.search_stack.as_slice());

        let mut movegen = MainMoveGenerator::new(
            maybe_tt_move,
            self.killer_moves[depth as usize],
            &prev_to,
            phase,
            self.config.get_piece_values(),
            self.config.quiet_check_bonus.value,
            threats,
        );

        // Used for punishing potentially "bad" quiet moves that were searched before a potential beta cutoff
        let mut quiets_searched: ArrayVec<ChessMove, { MAX_QUIETS }> = ArrayVec::new();
        let mut captures_searched: ArrayVec<ChessMove, { MAX_CAPTURES }> = ArrayVec::new();

        let mut move_index = -1;
        while let Some(m) = movegen.next(
            board,
            &self.history_heuristic,
            &self.capture_history,
            &self.continuation_history,
        ) {
            move_index += 1;

            // Late Move Pruning (LMP)
            if should_lmp_prune(
                board,
                m,
                in_check,
                is_pv_node,
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

            if let Some((value, mut line, is_quiet, child_depth)) = self.search_move(
                board,
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
                    best_move_depth = child_depth;
                }

                alpha = alpha.max(best_value);
                if alpha >= beta {
                    self.on_fail_high(
                        board,
                        m,
                        remaining_depth,
                        depth as usize,
                        is_quiet,
                        &quiets_searched,
                        &captures_searched,
                        threats,
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

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn search_move(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        in_check: bool,
        remaining_depth: u8,
        m: ChessMove,
        move_index: i32,
        is_improving: bool,
        static_eval: i16,
        pre_move_threats: BitBoard,
    ) -> Option<(i16, Vec<ChessMove>, bool, u8)> {
        let moved_piece = board.piece_on(m.get_source()).unwrap();
        let new_board = board.make_move_new(m);
        let child_hash = new_board.get_hash();

        // Prefetch TT entry for child position to hide memory latency
        self.tt.prefetch(child_hash);

        let gives_check = new_board.checkers().popcnt() > 0;

        // Consider move tactical if it's check, capture, or promotion
        let is_capture = board.piece_on(m.get_dest()).is_some();
        let is_promotion = m.get_promotion() == Some(Piece::Queen);
        let is_tactical = in_check || gives_check || is_capture || is_promotion;

        // Futility prune
        if self.try_futility_prune(remaining_depth, in_check, is_tactical, alpha, static_eval) {
            return None;
        }

        // Late move reduction
        let is_pv_move = move_index == 0;
        let mut reduction = lmr(
            remaining_depth,
            is_tactical,
            move_index,
            is_pv_move,
            is_improving,
            self.config.lmr_min_depth.value,
            self.config.lmr_divisor.value as f32 / 100.0,
            self.config.lmr_max_reduction_ratio.value as f32 / 100.0,
        );

        let alpha_child = alpha;
        let beta_child = if !is_pv_move { alpha + 1 } else { beta };

        // History-leaf pruning / extra reduction on quiet late moves
        if self.history_heuristic.maybe_reduce_or_prune(
            board,
            m,
            depth,
            max_depth,
            remaining_depth,
            in_check,
            is_tactical,
            is_pv_move,
            move_index,
            is_improving,
            &mut reduction,
            pre_move_threats,
        ) {
            return None;
        }

        let child_max_depth = max_depth.saturating_sub(reduction).max(depth + 1);
        let mut actual_depth = child_max_depth;

        self.search_stack.push_move(child_hash, m, moved_piece);
        let (child_value, pv_line) = self.search_subtree(
            &new_board,
            depth + 1,
            child_max_depth,
            -beta_child,
            -alpha_child,
            true,
            true,
        );
        self.search_stack.pop();
        let mut value = -child_value;
        let mut line = pv_line;

        if reduction > 0 && value > alpha {
            self.search_stack
                .push(SearchNode::with_move(child_hash, m, moved_piece));
            let (re_child_value, re_line) = self.search_subtree(
                &new_board,
                depth + 1,
                max_depth,
                -beta_child,
                -alpha_child,
                true,
                true,
            );
            self.search_stack.pop();
            value = -re_child_value;
            line = re_line;
            actual_depth = max_depth;
        }

        if !is_pv_move && value > alpha {
            self.search_stack
                .push(SearchNode::with_move(child_hash, m, moved_piece));
            let (full_child_value, full_line) =
                self.search_subtree(&new_board, depth + 1, max_depth, -beta, -alpha, true, true);
            self.search_stack.pop();
            value = -full_child_value;
            line = full_line;
            actual_depth = max_depth;
        }

        let is_quiet = !is_capture && !is_promotion;
        Some((value, line, is_quiet, actual_depth))
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn on_fail_high(
        &mut self,
        board: &Board,
        mv: ChessMove,
        remaining_depth: u8,
        depth: usize,
        is_quiet: bool,
        quiets_searched: &[ChessMove],
        captures_searched: &[ChessMove],
        threats: BitBoard,
    ) {
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
