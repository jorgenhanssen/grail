use crate::EngineConfig;
use crate::{
    negamax::{
        aspiration::{AspirationWindow, Pass},
        utils::{
            can_delta_prune, can_futility_prune, can_null_move_prune, can_razor_prune,
            can_reverse_futility_prune, futility_margin, razor_margin, rfp_margin,
        },
    },
    utils::{
        convert_centipawn_score, convert_mate_score, game_phase, see, CaptureHistory,
        ContinuationHistory, HistoryHeuristic, MainMoveGenerator, QMoveGenerator, MAX_CAPTURES,
        MAX_QUIETS,
    },
    Engine,
};
use arrayvec::ArrayVec;
use chess::{get_rank, BitBoard, Board, BoardStatus, ChessMove, Color, Piece, Rank};
use evaluation::PieceValues;
use evaluation::{
    hce,
    scores::{MATE_VALUE, NEG_INFINITY},
    HCE, NNUE,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use uci::{
    commands::{GoParams, Info, Score},
    UciOutput,
};

use super::utils::{
    improving, lmr, mate_distance_prune, null_move_reduction, should_lmp_prune, RAZOR_NEAR_MATE,
};
use super::{
    controller::SearchController,
    qs_table::QSTable,
    tt_table::{Bound, TranspositionTable},
};

const MAX_DEPTH: usize = 100;

pub struct NegamaxEngine {
    config: EngineConfig,
    piece_values: PieceValues,

    hce: Box<dyn HCE>,
    nnue: Option<Box<dyn NNUE>>,

    board: Board,
    nodes: u32,
    killer_moves: [[Option<ChessMove>; 2]; MAX_DEPTH], // 2 per depth
    current_pv: Vec<ChessMove>,
    max_depth_reached: u8,
    stop: Arc<AtomicBool>,

    tt: TranspositionTable,
    qs_tt: QSTable,

    position_stack: Vec<u64>,
    move_stack: Vec<ChessMove>,

    history_heuristic: HistoryHeuristic,
    capture_history: CaptureHistory,
    continuation_history: Box<ContinuationHistory>,

    eval_stack: Vec<i16>,
}

impl Engine for NegamaxEngine {
    fn new(config: &EngineConfig, hce: Box<dyn HCE>, nnue: Option<Box<dyn NNUE>>) -> Self {
        let mut instance = Self {
            config: config.clone(),
            piece_values: config.get_piece_values(),
            stop: Arc::new(AtomicBool::new(false)),

            hce,
            nnue,

            board: Board::default(),
            nodes: 0,
            killer_moves: [[None; 2]; MAX_DEPTH],
            current_pv: Vec::new(),
            max_depth_reached: 1,

            tt: TranspositionTable::new(1),
            qs_tt: QSTable::new(1),

            move_stack: Vec::with_capacity(MAX_DEPTH),
            eval_stack: Vec::with_capacity(MAX_DEPTH),
            position_stack: Vec::with_capacity(MAX_DEPTH),

            history_heuristic: HistoryHeuristic::new(1, 1, 1, 1, 1, 1),
            capture_history: CaptureHistory::new(1, 1, 1),
            continuation_history: Box::new(ContinuationHistory::new(1, 1, 1, 1)),
        };

        instance.configure(config, true);

        instance
    }

    fn configure(&mut self, config: &EngineConfig, init: bool) {
        let old_config = self.config.clone();
        self.config = config.clone();

        // Update the HCE
        // TODO: Find a better way to do this
        self.piece_values = config.get_piece_values();
        self.hce = Box::new(hce::Evaluator::new(
            self.piece_values,
            config.get_hce_config(),
        ));

        if init || old_config.hash_size.value != config.hash_size.value {
            self.configure_transposition_tables();
        }

        if init || !self.history_heuristic.matches_config(config) {
            self.history_heuristic.configure(config);
        }

        if init || !self.capture_history.matches_config(config) {
            self.capture_history.configure(config);
        }

        if init || !self.continuation_history.matches_config(config) {
            self.continuation_history.configure(config);
        }
    }

    fn name(&self) -> String {
        if let Some(nnue) = &self.nnue {
            format!("Negamax ({})", nnue.name())
        } else {
            format!("Negamax ({})", self.hce.name())
        }
    }

    fn new_game(&mut self) {
        self.init_game();
    }

    fn set_position(&mut self, board: Board) {
        self.board = board;
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn search(
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
}

impl NegamaxEngine {
    #[inline(always)]
    fn eval(&mut self, position: &utils::Position, phase: f32) -> i16 {
        if let Some(nnue) = &mut self.nnue {
            nnue.evaluate(position.board)
        } else {
            self.hce.evaluate(position, phase)
        }
    }

    #[inline(always)]
    pub fn init_game(&mut self) {
        self.tt.clear();
        self.qs_tt.clear();
        self.history_heuristic.reset();
        self.capture_history.reset();
        self.continuation_history.reset();
        self.eval_stack.clear();
    }

    #[inline(always)]
    pub fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);

        self.nodes = 0;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        self.position_stack.clear();
        self.position_stack.push(self.board.get_hash());
        self.move_stack.clear();
        self.eval_stack.clear();

        self.tt.age();
    }

    pub fn search_root(
        &mut self,
        depth: u8,
        mut alpha: i16,
        beta: i16,
    ) -> (Option<ChessMove>, i16) {
        let best_move = self.current_pv.first().cloned();

        let position = utils::Position::new(&self.board);
        let threats = position.threats_for(self.board.side_to_move());

        let prev_to = self
            .continuation_history
            .get_prev_to_squares(&self.move_stack);
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
            let new_board = self.board.make_move_new(m);

            self.position_stack.push(new_board.get_hash());
            self.move_stack.push(m);
            let (child_value, mut pv) =
                self.search_subtree(&new_board, 1, depth, -beta, -alpha, true, true);
            let score = -child_value;
            self.position_stack.pop();
            self.move_stack.pop();

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
    fn search_subtree(
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

        let hash = *self.position_stack.last().unwrap();
        if self.is_cycle(hash) {
            return (0, Vec::new()); // repetition = draw
        }

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

        let position = utils::Position::new(board);

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

        self.eval_stack.push(static_eval);
        let is_improving = !in_check && improving(static_eval, &self.eval_stack);

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
            .get_prev_to_squares(&self.move_stack);

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

        self.eval_stack.pop();

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
    fn search_move(
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

        self.position_stack.push(new_board.get_hash());
        self.move_stack.push(m);
        let (child_value, pv_line) = self.search_subtree(
            &new_board,
            depth + 1,
            child_max_depth,
            -beta_child,
            -alpha_child,
            true,
            true,
        );
        self.move_stack.pop();
        let mut value = -child_value;
        let mut line = pv_line;

        if reduction > 0 && value > alpha {
            self.move_stack.push(m);
            let (re_child_value, re_line) = self.search_subtree(
                &new_board,
                depth + 1,
                max_depth,
                -beta_child,
                -alpha_child,
                true,
                true,
            );
            self.move_stack.pop();
            value = -re_child_value;
            line = re_line;
            actual_depth = max_depth;
        }

        if !is_pv_move && value > alpha {
            self.move_stack.push(m);
            let (full_child_value, full_line) =
                self.search_subtree(&new_board, depth + 1, max_depth, -beta, -alpha, true, true);
            self.move_stack.pop();
            value = -full_child_value;
            line = full_line;
            actual_depth = max_depth;
        }

        self.position_stack.pop();

        let is_quiet = !is_capture && !is_promotion;
        Some((value, line, is_quiet, actual_depth))
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: i16,
        mut beta: i16,
        depth: u8,
    ) -> (i16, Vec<ChessMove>) {
        // Check if we should stop searching
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }

        self.nodes += 1;
        self.max_depth_reached = self.max_depth_reached.max(depth);

        let hash = *self.position_stack.last().unwrap();
        if self.is_cycle(hash) {
            return (0, Vec::new()); // Treat as a draw
        }

        if mate_distance_prune(&mut alpha, &mut beta, depth) {
            return (alpha, Vec::new());
        }

        let in_check = board.checkers().popcnt() > 0;

        let original_alpha = alpha;
        let original_beta = beta;

        if let Some((cached_value, cached_bound)) = self.qs_tt.probe(hash, in_check) {
            match cached_bound {
                Bound::Exact => return (cached_value, Vec::new()),
                Bound::Lower if cached_value >= beta => return (cached_value, Vec::new()),
                Bound::Upper if cached_value <= alpha => return (cached_value, Vec::new()),
                _ => {}
            }
        }

        let phase = game_phase(board);
        let position = utils::Position::new(board);

        let eval = self.eval(&position, phase);
        let stand_pat = if board.side_to_move() == Color::White {
            eval
        } else {
            -eval
        };

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
                let pawns = board.pieces(Piece::Pawn) & board.color_combined(board.side_to_move());
                let rank_mask = get_rank(promotion_rank);
                let promoting_pawns = pawns & rank_mask;

                if promoting_pawns != chess::EMPTY {
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
        let mut best_eval = if in_check { NEG_INFINITY } else { stand_pat };

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
                let captured = board.piece_on(mv.get_dest());
                if let Some(piece) = captured {
                    let mut delta =
                        self.piece_values.get(piece, phase) + self.config.qs_delta_margin.value;
                    if let Some(promotion) = mv.get_promotion() {
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
                if let Some(victim) = board.piece_on(mv.get_dest()) {
                    if let Some(attacker) = board.piece_on(mv.get_source()) {
                        let victim_value = self.piece_values.get(victim, phase);
                        let attacker_value = self.piece_values.get(attacker, phase);
                        // Only run expensive SEE if capture seems questionable (equal/lower value)
                        if victim_value <= attacker_value
                            && see(board, mv, phase, &self.piece_values) < 0
                        {
                            continue;
                        }
                    }
                }
            }

            let new_board = board.make_move_new(mv);
            let child_hash = new_board.get_hash();

            // Prefetch QS TT entry to hide memory latency
            self.qs_tt.prefetch(child_hash);

            self.position_stack.push(child_hash);
            let (child_score, mut child_line) =
                self.quiescence_search(&new_board, -beta, -alpha, depth + 1);
            self.position_stack.pop();

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
        if in_check && best_eval == NEG_INFINITY {
            return (-(MATE_VALUE - depth as i16), Vec::new());
        }

        self.qs_tt
            .store(hash, best_eval, original_alpha, original_beta, in_check);
        (best_eval, best_line)
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn on_fail_high(
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
            .get_prev_to_squares(&self.move_stack);
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

    #[inline(always)]
    fn is_cycle(&self, hash: u64) -> bool {
        self.position_stack.iter().filter(|&&h| h == hash).count() > 1
    }

    fn send_search_info(
        &self,
        output: &Sender<UciOutput>,
        current_depth: u8,
        best_score: i16,
        elapsed: std::time::Duration,
    ) {
        let found_checkmate = best_score.abs() >= MATE_VALUE - MAX_DEPTH as i16;
        let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

        output
            .send(UciOutput::Info(Info {
                depth: current_depth,
                sel_depth: self.max_depth_reached,
                nodes: self.nodes,
                nodes_per_second: nps,
                time: elapsed.as_millis() as u32,
                score: if found_checkmate {
                    convert_mate_score(best_score)
                } else {
                    convert_centipawn_score(best_score)
                },
                pv: self.current_pv.clone(),
            }))
            .unwrap();
    }

    #[inline(always)]
    fn try_futility_prune(
        &self,
        remaining_depth: u8,
        in_check: bool,
        is_tactical: bool,
        alpha: i16,
        static_eval: i16,
    ) -> bool {
        if !can_futility_prune(
            remaining_depth,
            in_check,
            self.config.futility_max_depth.value,
        ) {
            return false;
        }
        let margin = futility_margin(
            remaining_depth,
            self.config.futility_base_margin.value,
            self.config.futility_depth_multiplier.value,
        );
        !is_tactical && static_eval + margin <= alpha
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn try_razor_prune(
        &mut self,
        board: &Board,
        remaining_depth: u8,
        alpha: i16,
        depth: u8,
        in_check: bool,
        static_eval: i16,
    ) -> Option<i16> {
        if !can_razor_prune(remaining_depth, in_check, self.config.razor_max_depth.value) {
            return None;
        }
        // If static eval already near/above alpha threshold, do not razor
        let margin = razor_margin(
            remaining_depth,
            self.config.razor_base_margin.value,
            self.config.razor_depth_coefficient.value,
        );
        if static_eval >= alpha - margin {
            return None;
        }
        // Q search with null window
        let (value, _) = self.quiescence_search(board, alpha - 1, alpha, depth);
        if value < alpha && value.abs() < RAZOR_NEAR_MATE {
            Some(value)
        } else {
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn try_null_move_prune(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        hash: u64,
        remaining_depth: u8,
        in_check: bool,
        try_null_move: bool,
        static_eval: Option<i16>,
    ) -> Option<i16> {
        // Null move pruning: if giving the opponent a free move still doesn't let
        // them reach beta, the position is strong enough to prune

        if !(try_null_move
            && can_null_move_prune(
                board,
                remaining_depth,
                in_check,
                self.config.nmp_min_depth.value,
            ))
        {
            return None;
        }
        let nm_board = board.null_move()?;
        let base_remaining = max_depth - depth;

        // Calculate reduction based on remaining depth and static eval
        let r = null_move_reduction(
            base_remaining,
            static_eval,
            beta,
            self.config.nmp_base_reduction.value,
            self.config.nmp_depth_divisor.value,
            self.config.nmp_eval_margin.value,
        );

        // Do a reduced depth null search to check if our position is still good enough
        self.position_stack.push(nm_board.get_hash());
        let (score, _) = self.search_subtree(
            &nm_board,
            depth + 1,
            max_depth - r,
            -beta,
            -beta + 1,
            false,
            false,
        );
        self.position_stack.pop();

        // The opponent still can't reach beta,
        // so the position is strong enough to prune
        if -score >= beta {
            // However, in Zugzwang positions, passing is better than any legal move
            // so we need to verify that the position is still good enough
            if base_remaining <= 6 {
                self.position_stack.push(nm_board.get_hash());
                let verify_depth = max_depth - r.saturating_sub(1);
                let (v_score, _) = self.search_subtree(
                    &nm_board,
                    depth + 1,
                    verify_depth,
                    -beta,
                    -beta + 1,
                    false,
                    false,
                );
                self.position_stack.pop();
                if -v_score < beta {
                    return None; // fail verification; do not prune
                }
            }

            let null_move_depth = max_depth - r;
            self.tt
                .store(hash, depth, null_move_depth, beta, None, alpha, beta, None);
            return Some(beta);
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn try_reverse_futility_prune(
        &mut self,
        remaining_depth: u8,
        in_check: bool,
        is_pv_node: bool,
        static_eval: i16,
        beta: i16,
        hash: u64,
        depth: u8,
        _max_depth: u8,
        alpha: i16,
        is_improving: bool,
    ) -> Option<i16> {
        if !can_reverse_futility_prune(
            remaining_depth,
            in_check,
            is_pv_node,
            self.config.rfp_max_depth.value,
        ) {
            return None;
        }

        let margin = rfp_margin(
            remaining_depth,
            self.config.rfp_base_margin.value,
            self.config.rfp_depth_multiplier.value,
            is_improving,
            self.config.rfp_improving_bonus.value,
        );
        if static_eval - margin >= beta && static_eval.abs() < RAZOR_NEAR_MATE {
            let rfp_depth = depth;
            self.tt.store(
                hash,
                depth,
                rfp_depth,
                beta,
                Some(static_eval),
                alpha,
                beta,
                None,
            );
            return Some(beta);
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn try_iid(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        try_null_move: bool,
        allow_iid: bool,
        need_iid: bool,
        remaining_depth: u8,
        in_check: bool,
    ) -> Option<ChessMove> {
        // Gate
        if !(allow_iid && need_iid && remaining_depth >= 4 && !in_check) {
            return None;
        }
        let shallow_max = max_depth.saturating_sub(self.config.iid_reduction.value);
        let (.., shallow_line) = self.search_subtree(
            board,
            depth,
            shallow_max,
            alpha,
            beta,
            try_null_move,
            false, // disable nested IID
        );
        shallow_line.first().copied()
    }
}

impl NegamaxEngine {
    fn configure_transposition_tables(&mut self) {
        let total_size_mb = self.config.hash_size.value;
        let qs_size_mb = total_size_mb / 3;
        let main_size_mb = total_size_mb - qs_size_mb;

        self.tt = TranspositionTable::new(main_size_mb as usize);
        self.qs_tt = QSTable::new(qs_size_mb as usize);
    }
}
