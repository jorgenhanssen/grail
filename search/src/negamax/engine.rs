use crate::{
    negamax::{
        aspiration::{
            AspirationWindow, Pass, ASP_ENABLED_FROM, ASP_HALF_START, ASP_MAX_RETRIES, ASP_WIDEN,
        },
        utils::{can_delta_prune, can_null_move_prune},
    },
    utils::{ordered_moves, Castle, CAPTURE_PRIORITY, MAX_PRIORITY},
    Engine,
};
use ahash::AHashMap;
use chess::{get_rank, Board, BoardStatus, ChessMove, Color, Piece, Rank};
use evaluation::{
    piece_value,
    scores::{MATE_VALUE, NEG_INFINITY, POS_INFINITY},
    PAWN_VALUE, QUEEN_VALUE,
};
use evaluation::{Evaluator, TraditionalEvaluator};
use std::array;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use uci::{
    commands::{GoParams, Info, Score},
    UciOutput,
};

use super::utils::{convert_centipawn_score, convert_mate_score, lmr, see_naive};
use super::{
    controller::SearchController,
    tt::{Bound, TTEntry},
};

const MAX_DEPTH: usize = 100;

pub struct NegamaxEngine {
    board: Board,
    nodes: u32,
    killer_moves: [[Option<ChessMove>; 2]; MAX_DEPTH], // 2 per depth
    current_pv: Vec<ChessMove>,

    window: AspirationWindow,
    tt: AHashMap<u64, TTEntry>,
    qs_tt: AHashMap<u64, i16>,

    max_depth_reached: u8,

    // move sorting buffer per depth
    preferred_buffer: [Vec<(ChessMove, i16)>; MAX_DEPTH],

    position_stack: Vec<u64>,
    evaluator: Box<dyn Evaluator>,

    stop: Arc<AtomicBool>,
}

impl Default for NegamaxEngine {
    fn default() -> Self {
        Self {
            board: Board::default(),
            nodes: 0,
            window: AspirationWindow::new(ASP_HALF_START, ASP_WIDEN, ASP_ENABLED_FROM),
            tt: AHashMap::with_capacity(200_000),
            qs_tt: AHashMap::with_capacity(100_000),
            killer_moves: [[None; 2]; MAX_DEPTH],
            max_depth_reached: 1,
            current_pv: Vec::new(),
            preferred_buffer: array::from_fn(|_| Vec::with_capacity(MAX_DEPTH)),

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

        let mut controller = SearchController::new(params);
        let stop = Arc::clone(&self.stop);
        controller.on_stop(move || stop.store(true, Ordering::Relaxed));
        controller.start_timer();

        let mut depth = 1;
        let mut best_move = None;
        let mut best_score = 0;

        while !self.stop.load(Ordering::Relaxed) && depth <= MAX_DEPTH as u8 {
            controller.check_depth(depth);
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            self.window.begin_depth(depth, best_score);
            let mut retries = 0;

            loop {
                let (alpha, beta) = self.window.bounds();
                let (mv, score) = self.search_root(depth, alpha, beta);

                if mv.is_none() {
                    break;
                }

                match self.window.analyse_pass(score) {
                    Pass::Hit(s) => {
                        best_move = mv;
                        best_score = s;
                        if let Some(out) = output {
                            self.send_search_info(out, depth, s, controller.elapsed());
                        }
                        break;
                    }
                    _ => {
                        retries += 1;

                        if retries >= ASP_MAX_RETRIES {
                            self.window.fully_extend();
                            retries = 0;
                        }
                    }
                }
            }

            depth += 1;
        }

        if let Some(mv) = best_move {
            Some((mv, best_score))
        } else {
            None
        }
    }
}

impl NegamaxEngine {
    #[inline(always)]
    pub fn init_search(&mut self) {
        self.stop.store(false, Ordering::Relaxed);

        self.tt.clear();
        self.qs_tt.clear();

        self.killer_moves = [[None; 2]; MAX_DEPTH]; // 2 killer moves per depth
        self.nodes = 0;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        // Init position stack
        self.position_stack.clear();
        self.position_stack.push(self.board.get_hash());
    }

    pub fn search_root(
        &mut self,
        depth: u8,
        mut alpha: i16,
        beta: i16,
    ) -> (Option<ChessMove>, i16) {
        let pref = &mut self.preferred_buffer[0];
        pref.clear();

        if let Some(&pv) = self.current_pv.first() {
            pref.push((pv, POS_INFINITY));
        }
        let moves_with_scores = ordered_moves(&self.board, Some(&pref[..]), None);

        if moves_with_scores.is_empty() {
            return (None, 0);
        }

        let mut best_score = NEG_INFINITY;
        let mut current_best_move = None;

        // Negamax at root: call search_subtree with flipped window, then negate result
        for (m, _) in moves_with_scores {
            let castle = Castle::new().update(&self.board, m);

            let new_board = self.board.make_move_new(m);

            self.position_stack.push(new_board.get_hash());
            let (child_value, mut pv) =
                self.search_subtree(&new_board, 1, depth, -beta, -alpha, true, castle);
            let score = -child_value;
            self.position_stack.pop();

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

    fn search_subtree(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        mut alpha: i16,
        beta: i16,
        try_null_move: bool,
        castle: Castle,
    ) -> (i16, Vec<ChessMove>) {
        if self.stop.load(Ordering::Relaxed) {
            return (0, Vec::new());
        }
        self.nodes += 1;

        let hash = *self.position_stack.last().unwrap();
        if self.is_cycle(hash) {
            return (0, Vec::new()); // repetition = draw
        }

        // Terminal checks
        match board.status() {
            BoardStatus::Checkmate => return (-MATE_VALUE + depth as i16, Vec::new()),
            BoardStatus::Stalemate => return (0, Vec::new()),
            BoardStatus::Ongoing => {}
        }
        if depth >= max_depth {
            return self.quiescence_search(board, alpha, beta, depth, castle);
        }

        // Transposition table probe
        let original_alpha = alpha;
        let mut maybe_tt_move = None;
        if let Some((tt_value, tt_bound, tt_move)) = self.probe_tt(hash, depth, max_depth) {
            maybe_tt_move = tt_move;
            match tt_bound {
                Bound::Exact => return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m])),
                Bound::Lower => alpha = alpha.max(tt_value),
                Bound::Upper => {
                    if tt_value <= alpha {
                        return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m]));
                    }
                }
            }
            if alpha >= beta {
                return (tt_value, tt_move.map_or(Vec::new(), |m| vec![m]));
            }
        }

        // Null-move pruning
        let remaining_depth = max_depth - depth;
        let in_check = board.checkers().popcnt() > 0;

        if try_null_move && can_null_move_prune(board, remaining_depth, in_check) {
            if let Some(score) =
                self.null_move_prune(board, depth, max_depth, alpha, beta, hash, castle)
            {
                return (score, Vec::new());
            }
        }

        self.max_depth_reached = self.max_depth_reached.max(depth);

        // Move ordering
        let pref = &mut self.preferred_buffer[depth as usize];
        pref.clear();

        if let Some(&move_) = self.current_pv.get(depth as usize) {
            pref.push((move_, MAX_PRIORITY + 2));
        }

        if let Some(tt_move) = maybe_tt_move {
            pref.push((tt_move, MAX_PRIORITY + 1));
        }

        for &killer_move_opt in &self.killer_moves[depth as usize] {
            if let Some(killer_move) = killer_move_opt {
                let already_there = pref.iter().any(|&(pm, _)| pm == killer_move);
                if !already_there {
                    pref.push((killer_move, CAPTURE_PRIORITY - 1));
                }
            }
        }

        let moves = ordered_moves(board, Some(&pref[..]), None);

        if moves.is_empty() {
            return (0, Vec::new());
        }

        let mut best_value = NEG_INFINITY;
        let mut best_move = None;
        let mut best_line = Vec::new();

        for (m, score) in moves {
            let new_castle = castle.update(board, m);

            let new_board = board.make_move_new(m);
            let gives_check = new_board.checkers().popcnt() > 0;

            let reduction = lmr(remaining_depth, score, in_check || gives_check);
            let child_max_depth = max_depth.saturating_sub(reduction).max(depth + 1);

            self.position_stack.push(new_board.get_hash());

            let (child_value, mut line) = self.search_subtree(
                &new_board,
                depth + 1,
                child_max_depth,
                -beta,
                -alpha,
                true,
                new_castle,
            );
            let mut value = -child_value;

            // Re-search at full depth if reduced search failed high
            if reduction > 0 && value > alpha {
                let (re_child_value, re_line) = self.search_subtree(
                    &new_board,
                    depth + 1,
                    max_depth,
                    -beta,
                    -alpha,
                    true,
                    new_castle,
                );
                value = -re_child_value;
                line = re_line;
            }

            self.position_stack.pop();

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
                if let Some(mv) = best_move {
                    if board.piece_on(mv.get_dest()).is_none() {
                        self.add_killer_move(depth as usize, mv);
                    }
                }
                break; // beta cut-off
            }
        }

        self.store_tt(
            hash,
            depth,
            max_depth,
            best_value,
            original_alpha,
            beta,
            best_move,
        );
        (best_value, best_line)
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: i16,
        beta: i16,
        depth: u8,
        castle: Castle,
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

        match board.status() {
            BoardStatus::Checkmate => {
                return (-MATE_VALUE + depth as i16, Vec::new());
            }
            BoardStatus::Stalemate => {
                return (0, Vec::new());
            }
            BoardStatus::Ongoing => {}
        }

        // Check cache
        if let Some(&cached_score) = self.qs_tt.get(&hash) {
            return (cached_score, Vec::new());
        }

        let eval = self.evaluator.evaluate(
            board,
            castle.white_has_castled(),
            castle.black_has_castled(),
        );
        let stand_pat = if board.side_to_move() == Color::White {
            eval
        } else {
            -eval
        };

        let in_check = board.checkers().popcnt() > 0;

        // Do a "stand-pat" evaluation if not in check
        if !in_check {
            if stand_pat >= beta {
                self.qs_tt.insert(hash, stand_pat);
                return (stand_pat, Vec::new());
            }

            // Node-level delta pruning (big delta)
            if can_delta_prune(board, in_check) {
                let mut big_delta = QUEEN_VALUE;
                let promotion_rank = if board.side_to_move() == Color::White {
                    Rank::Seventh
                } else {
                    Rank::Second
                };
                let pawns = board.pieces(Piece::Pawn) & board.color_combined(board.side_to_move());
                let rank_mask = get_rank(promotion_rank);
                let promoting_pawns = pawns & rank_mask;

                if promoting_pawns != chess::EMPTY {
                    big_delta += QUEEN_VALUE - PAWN_VALUE;
                }

                if stand_pat + big_delta < alpha {
                    self.qs_tt.insert(hash, stand_pat);
                    return (stand_pat, Vec::new());
                }
            }

            alpha = alpha.max(stand_pat);
        }

        let mut best_line = Vec::new();
        let mut best_eval = if in_check { NEG_INFINITY } else { stand_pat };

        let mask = if in_check {
            None // We should check all moves
        } else {
            // Include both captures and promotion squares
            let captures = *board.color_combined(!board.side_to_move());
            let promotion_rank = if board.side_to_move() == Color::White {
                get_rank(Rank::Eighth)
            } else {
                get_rank(Rank::First)
            };
            Some(captures | promotion_rank)
        };
        let forcing_moves = ordered_moves(board, None, mask);

        for (mv, _) in forcing_moves {
            // Per-move delta pruning (skip if capture can't possibly improve alpha)
            if can_delta_prune(board, in_check) {
                let captured = board.piece_on(mv.get_dest());
                if let Some(piece) = captured {
                    let mut delta = piece_value(piece) + 200; // delta margin
                    if mv.get_promotion().is_some() {
                        delta += QUEEN_VALUE - PAWN_VALUE; // promotion bonus
                    }
                    if stand_pat + delta < alpha {
                        continue;
                    }
                } else {
                    // Not a capture (should not happen with mask, but skip for safety)
                    continue;
                }
            }

            if !in_check && see_naive(board, mv) < 0 {
                continue;
            }

            let new_board = board.make_move_new(mv);

            self.position_stack.push(new_board.get_hash());
            let (child_score, mut child_line) =
                self.quiescence_search(&new_board, -beta, -alpha, depth + 1, castle);
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

        self.qs_tt.insert(hash, best_eval);
        (best_eval, best_line)
    }

    #[inline]
    fn probe_tt(
        &mut self,
        hash: u64,
        depth: u8,
        max_depth: u8,
    ) -> Option<(i16, Bound, Option<ChessMove>)> {
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
        depth: u8,
        max_depth: u8,
        value: i16,
        alpha: i16,
        beta: i16,
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

        let entry = TTEntry::new(plies, value, bound, best_move);

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
                    convert_mate_score(best_score, &self.current_pv)
                } else {
                    convert_centipawn_score(best_score)
                },
                pv: self.current_pv.clone(),
            }))
            .unwrap();
    }

    #[inline(always)]
    fn null_move_prune(
        &mut self,
        board: &Board,
        depth: u8,
        max_depth: u8,
        alpha: i16,
        beta: i16,
        hash: u64,
        castle: Castle,
    ) -> Option<i16> {
        // Null move pruning: if giving opponent a free move still can't reach beta, we can prune

        // Less reduction near horizon
        let r = match max_depth - depth {
            3..7 => 2,
            _ => 3,
        };

        if let Some(nm_board) = board.null_move() {
            // Give opponent extra move and search with reduced depth
            self.position_stack.push(nm_board.get_hash());
            let (score, _) = self.search_subtree(
                &nm_board,
                depth + 1,
                max_depth - r,
                -beta,
                -beta + 1, // null window
                false,     // Null moves cannot be done in sequence, so disable for next move
                castle,
            );
            self.position_stack.pop();

            // If opponent still can't reach beta, prune this branch
            if -score >= beta {
                self.store_tt(hash, depth, max_depth, beta, alpha, beta, None);
                return Some(beta);
            }
        }

        None
    }
}
