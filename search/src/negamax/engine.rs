use crate::{
    utils::{get_ordered_moves, CAPTURE_SCORE, CHECK_SCORE, PROMOTION_SCORE},
    Engine,
};
use ahash::AHashMap;
use chess::{Board, BoardStatus, ChessMove, Color};
use evaluation::{Evaluator, TraditionalEvaluator};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use uci::{
    commands::{GoParams, Info},
    UciOutput,
};

use super::utils::{
    calculate_dynamic_lmr_reduction, convert_centipawn_score, convert_mate_score, see_naive,
};
use super::{
    controller::SearchController,
    tt::{Bound, TTEntry},
};

pub const CHECKMATE_SCORE: f32 = 1_000_000.0;

const MAX_QSEARCH_DEPTH: u64 = 12;
const MAX_QSEARCH_CHECK_STREAK: u64 = 4;

pub struct NegamaxEngine {
    board: Board,
    nodes: u32,
    killer_moves: [[Option<ChessMove>; 2]; 100], // 2 per depth
    current_pv: Vec<ChessMove>,

    tt: AHashMap<u64, TTEntry>,
    qs_tt: AHashMap<u64, f32>,

    max_depth_reached: u64,

    position_stack: Vec<u64>,
    evaluator: Box<dyn Evaluator>,

    stop: Arc<AtomicBool>,
}

impl Default for NegamaxEngine {
    fn default() -> Self {
        Self {
            board: Board::default(),
            nodes: 0,
            tt: AHashMap::with_capacity(6_000_000),
            qs_tt: AHashMap::with_capacity(12_000_000),
            killer_moves: [[None; 2]; 100], // 100 is a good depth
            max_depth_reached: 1,
            current_pv: Vec::new(),

            position_stack: Vec::with_capacity(100),
            evaluator: Box::new(TraditionalEvaluator),

            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Engine for NegamaxEngine {
    fn new(evaluator: Box<dyn Evaluator>) -> Self {
        Self {
            evaluator,
            stop: Arc::new(AtomicBool::new(false)),
            ..Default::default()
        }
    }

    fn name(&self) -> String {
        format!("Negamax ({})", self.evaluator.name())
    }

    fn set_position(&mut self, board: Board) {
        self.board = board;
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn search(&mut self, params: &GoParams, output: &Sender<UciOutput>) -> ChessMove {
        self.init_search();

        let mut controller = SearchController::new(params);

        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || {
            stop.store(true, Ordering::Relaxed);
        });

        controller.start_timer();

        let mut depth = 1;
        let mut best_move = None;

        while !self.stop.load(Ordering::Relaxed) {
            // Check depth limits - this will call on_stop if max depth reached
            controller.check_depth(depth);

            // Stop if the depth check set the stop flag
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            let (mv, score) = self.search_root(depth);

            if mv.is_none() {
                break;
            }

            best_move = mv;

            self.send_search_info(output, depth, score, controller.elapsed());

            depth += 1;
        }

        best_move.unwrap()
    }
}

impl NegamaxEngine {
    #[inline(always)]
    pub fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);
        self.tt.clear();
        self.qs_tt.clear();
        self.killer_moves = [[None; 2]; 100]; // 2 killer moves per depth
        self.nodes = 0;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        // Init position stack
        self.position_stack.clear();
        self.position_stack.push(self.board.get_hash());
    }

    pub fn search_root(&mut self, depth: u64) -> (Option<ChessMove>, f32) {
        let mut alpha = f32::NEG_INFINITY;
        let beta = f32::INFINITY;

        let mut preferred_moves = Vec::with_capacity(1);
        if let Some(move_) = self.current_pv.first() {
            preferred_moves.push((*move_, i32::MAX));
        }
        let moves_with_scores = get_ordered_moves(&self.board, Some(&preferred_moves));

        if moves_with_scores.is_empty() {
            return (None, 0.0);
        }

        let mut best_score = f32::NEG_INFINITY;
        let mut current_best_move = None;

        // Negamax at root: call search_subtree with flipped window, then negate result
        for (m, _) in moves_with_scores {
            let new_board = self.board.make_move_new(m);

            self.position_stack.push(new_board.get_hash());
            let (child_value, mut pv) = self.search_subtree(&new_board, 1, depth, -beta, -alpha);
            let score = -child_value;
            self.position_stack.pop();

            // Check if we were stopped during the subtree search
            if self.stop.load(Ordering::Relaxed) {
                return (None, 0.0);
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

    fn search_subtree(
        &mut self,
        board: &Board,
        depth: u64,
        max_depth: u64,
        mut alpha: f32,
        beta: f32,
    ) -> (f32, Vec<ChessMove>) {
        // Check if we should stop searching
        if self.stop.load(Ordering::Relaxed) {
            return (0.0, Vec::new());
        }

        let hash = *self.position_stack.last().unwrap();

        if self.is_cycle(hash) {
            return (0.0, Vec::new()); // Treat as a draw
        }

        match board.status() {
            BoardStatus::Checkmate => {
                self.nodes += 1;
                let remaining_depth = (max_depth - depth) as f32;
                return (-CHECKMATE_SCORE * (remaining_depth + 1.0), Vec::new());
            }
            BoardStatus::Stalemate => {
                self.nodes += 1;
                return (0.0, Vec::new());
            }
            BoardStatus::Ongoing => {}
        }

        if depth >= max_depth {
            return self.quiescence_search(board, alpha, beta, 1, 0);
        }

        let mut maybe_tt_move = None;
        if let Some((tt_value, tt_bound, tt_move)) = self.probe_tt(hash, depth, max_depth) {
            maybe_tt_move = tt_move;
            match tt_bound {
                Bound::Exact => {
                    return (tt_value, maybe_tt_move.map_or(Vec::new(), |m| vec![m]));
                }
                Bound::Lower => {
                    if tt_value > alpha {
                        alpha = tt_value;
                    }
                }
                Bound::Upper => {
                    if tt_value <= alpha {
                        return (tt_value, maybe_tt_move.map_or(Vec::new(), |m| vec![m]));
                    }
                }
            }
            if alpha >= beta {
                return (tt_value, maybe_tt_move.map_or(Vec::new(), |m| vec![m]));
            }
        }

        self.nodes += 1;
        self.max_depth_reached = self.max_depth_reached.max(depth);

        let mut preferred_moves = Vec::with_capacity(5);

        // First priority is the current PV move
        if let Some(&move_) = self.current_pv.get(depth as usize) {
            preferred_moves.push((move_, i32::MAX));
        }

        // Then any hash move from the tt
        if maybe_tt_move.is_some() {
            preferred_moves.push((maybe_tt_move.unwrap(), i32::MAX - 1));
        }

        //  Killer moves for this specific depth if they are legal
        for &killer_move_opt in &self.killer_moves[depth as usize] {
            if let Some(killer_move) = killer_move_opt {
                let already_there = preferred_moves.iter().any(|&(pm, _)| pm == killer_move);
                if !already_there {
                    preferred_moves.push((killer_move, CAPTURE_SCORE - 2));
                }
            }
        }

        let moves = get_ordered_moves(board, Some(&preferred_moves));

        if moves.is_empty() {
            return (0.0, Vec::new());
        }

        // Negamax
        let mut best_value = f32::NEG_INFINITY;
        let mut best_move = None;
        let mut best_line = Vec::new();

        for (move_index, (m, score)) in moves.into_iter().enumerate() {
            let reduction = calculate_dynamic_lmr_reduction(depth, move_index, score);
            let current_max_depth = max_depth.saturating_sub(reduction).max(depth + 1);

            let new_board = board.make_move_new(m);

            self.position_stack.push(new_board.get_hash());
            let (child_value, mut line) =
                self.search_subtree(&new_board, depth + 1, current_max_depth, -beta, -alpha);
            let value = -child_value;
            self.position_stack.pop();

            // Check if we were stopped during the recursive search
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            if value > best_value {
                best_value = value;
                best_move = Some(m);
                line.insert(0, m);
                best_line = line;
            }

            alpha = alpha.max(best_value);
            if alpha >= beta {
                if let Some(m) = best_move {
                    if !board.piece_on(m.get_dest()).is_some() {
                        self.add_killer_move(depth as usize, m);
                    }
                }
                break;
            }
        }

        self.store_tt(hash, depth, max_depth, best_value, alpha, beta, best_move);
        (best_value, best_line)
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: f32,
        beta: f32,
        depth: u64,
        check_streak: u64,
    ) -> (f32, Vec<ChessMove>) {
        // Check if we should stop searching
        if self.stop.load(Ordering::Relaxed) {
            return (0.0, Vec::new());
        }

        let hash = *self.position_stack.last().unwrap();
        if self.is_cycle(hash) {
            return (0.0, Vec::new()); // Treat as a draw
        }

        self.nodes += 1;

        match board.status() {
            BoardStatus::Checkmate => {
                return (-CHECKMATE_SCORE, Vec::new());
            }
            BoardStatus::Stalemate => {
                return (0.0, Vec::new());
            }
            BoardStatus::Ongoing => {}
        }

        // Check cache
        if let Some(&cached_score) = self.qs_tt.get(&hash) {
            return (cached_score, Vec::new());
        }

        let color_multiplier = if board.side_to_move() == Color::White {
            1.0
        } else {
            -1.0
        };
        let stand_pat = color_multiplier * self.evaluator.evaluate(board);

        if depth >= MAX_QSEARCH_DEPTH || check_streak >= MAX_QSEARCH_CHECK_STREAK {
            self.qs_tt.insert(hash, stand_pat);
            return (stand_pat, Vec::new());
        }

        let in_check = board.checkers().popcnt() > 0;

        // Do a "stand-pat" evaluation if not in check
        if !in_check {
            if stand_pat >= beta {
                return (stand_pat, Vec::new());
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }
        }

        let all_moves_with_scores = get_ordered_moves(board, None);
        let forcing_moves: Vec<(ChessMove, i32)> = all_moves_with_scores
            .into_iter()
            .filter(|(mv, score)| {
                if in_check {
                    // Must consider all evasions if currently in check
                    return true;
                }
                // Otherwise, only consider captures/promo/strong checks, etc.
                if *score >= PROMOTION_SCORE || *score == CHECK_SCORE {
                    return true;
                }
                if *score >= CAPTURE_SCORE {
                    return see_naive(board, *mv) >= 0.0;
                }
                false
            })
            .collect();

        if forcing_moves.is_empty() && !in_check {
            // If in_check and no moves, then we are checkmated or stalemated
            // but that is already handled by board.status() above.
            self.qs_tt.insert(hash, stand_pat);
            return (stand_pat, Vec::new());
        }

        // 10) Try each forcing move and pick the best
        let mut best_eval = if in_check {
            // If we are in check, we can't simply "stand pat."
            // Let's start from -∞
            f32::NEG_INFINITY
        } else {
            // Otherwise, start from our stand-pat
            stand_pat
        };

        let mut best_line = Vec::new();

        for (mv, _) in forcing_moves {
            let new_board = board.make_move_new(mv);

            // Decide how to update check_streak for the child:
            let delivers_check = new_board.checkers().popcnt() > 0;
            let new_check_streak = if in_check || delivers_check {
                check_streak + 1
            } else {
                0
            };

            self.position_stack.push(new_board.get_hash());
            let (child_score, mut child_line) =
                self.quiescence_search(&new_board, -beta, -alpha, depth + 1, new_check_streak);
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
            }

            alpha = alpha.max(best_eval);
            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        self.qs_tt.insert(hash, best_eval);
        (best_eval, best_line)
    }

    #[inline]
    fn probe_tt(
        &mut self,
        hash: u64,
        depth: u64,
        max_depth: u64,
    ) -> Option<(f32, Bound, Option<ChessMove>)> {
        let plies = max_depth - depth;
        if let Some(entry) = self.tt.get(&hash) {
            if entry.plies >= plies {
                return Some((entry.value, entry.bound, entry.best_move));
            }
        }
        None
    }

    fn store_tt(
        &mut self,
        hash: u64,
        depth: u64,
        max_depth: u64,
        value: f32,
        alpha: f32,
        beta: f32,
        best_move: Option<ChessMove>,
    ) {
        let plies = max_depth - depth;

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        let entry = TTEntry {
            plies,
            value,
            bound,
            best_move,
        };

        if let Some(old_entry) = self.tt.get(&hash) {
            if old_entry.plies <= plies {
                self.tt.insert(hash, entry);
            }
        } else {
            self.tt.insert(hash, entry);
        }
    }

    #[inline(always)]
    fn add_killer_move(&mut self, depth: usize, m: ChessMove) {
        let killers = &mut self.killer_moves[depth];
        if killers[0] != Some(m) {
            killers[1] = killers[0];
            killers[0] = Some(m);
        }
    }

    #[inline(always)]
    fn is_cycle(&self, hash: u64) -> bool {
        self.position_stack.iter().filter(|&&h| h == hash).count() >= 2
    }

    fn send_search_info(
        &self,
        output: &Sender<UciOutput>,
        current_depth: u64,
        best_score: f32,
        elapsed: std::time::Duration,
    ) {
        let found_checkmate = best_score.abs() >= CHECKMATE_SCORE;
        let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

        output
            .send(UciOutput::Info(Info {
                depth: current_depth,
                sel_depth: self.max_depth_reached,
                nodes: self.nodes,
                nodes_per_second: nps,
                time: elapsed.as_millis() as u32,
                score: if found_checkmate {
                    convert_mate_score(&self.board, best_score, &self.current_pv)
                } else {
                    convert_centipawn_score(best_score)
                },
                pv: self.current_pv.clone(),
            }))
            .unwrap();
    }
}
