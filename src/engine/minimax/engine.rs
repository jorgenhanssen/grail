use crate::engine::Engine;
use crate::uci::commands::Score;
use crate::utils::{
    get_ordered_moves, piece_value, CAPTURE_SCORE, CHECKMATE_SCORE, CHECK_SCORE, PROMOTION_SCORE,
};
use crate::{
    uci::{
        commands::{GoParams, Info},
        UciOutput,
    },
    utils::evaluate_board,
};
use ahash::AHashMap;
use chess::{Board, BoardStatus, ChessMove};
use std::sync::mpsc::Sender;

use super::tt::{Bound, TTEntry};

pub struct MinimaxEngine {
    board: Board,
    nodes: u32,
    killer_moves: Vec<[Option<ChessMove>; 2]>, // 2 per depth
    current_pv: Vec<ChessMove>,

    tt: AHashMap<u64, TTEntry>,
    qs_tt: AHashMap<u64, f32>,

    search_depth: u32,
    max_depth_reached: u32,
}

impl Default for MinimaxEngine {
    fn default() -> Self {
        Self {
            board: Board::default(),
            nodes: 0,
            tt: AHashMap::with_capacity(64_000_000),
            qs_tt: AHashMap::with_capacity(128_000_000),
            killer_moves: vec![[None; 2]; 100], // 100 is a good depth
            search_depth: 1,
            max_depth_reached: 1,
            current_pv: Vec::new(),
        }
    }
}

impl Engine for MinimaxEngine {
    fn set_position(&mut self, board: Board) {
        self.board = board;
    }

    fn stop(&mut self) {
        // TODO: implement
    }

    fn search(&mut self, params: &GoParams, output: &Sender<UciOutput>) -> ChessMove {
        self.tt.clear();
        self.killer_moves = vec![[None; 2]; 100];
        self.nodes = 0;
        self.search_depth = 1;
        self.max_depth_reached = 1;
        self.current_pv.clear();

        let search_time = params.move_time.unwrap_or(10_000);
        let start_time = std::time::Instant::now();

        let mut best_move = None;

        while start_time.elapsed().as_millis() < search_time as u128 {
            let mut alpha = f32::NEG_INFINITY;
            let mut beta = f32::INFINITY;

            let mut preferred_moves = AHashMap::with_capacity(1);
            let best_first_move = self.current_pv.first();
            if let Some(move_) = best_first_move {
                preferred_moves.insert(move_.clone(), i32::MAX);
            }
            let moves_with_scores = get_ordered_moves(&self.board, Some(&preferred_moves));

            let maximizing = self.board.side_to_move() == chess::Color::White;
            let mut best_score = if maximizing {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
            let mut current_best_move = moves_with_scores[0].0;

            for (m, _) in moves_with_scores {
                let new_board = self.board.make_move_new(m);
                let (score, mut pv) = self.alpha_beta(&new_board, 1, alpha, beta);
                pv.insert(0, m); // Add current move to the beginning of the line

                if maximizing {
                    if score > best_score {
                        best_score = score;
                        current_best_move = m;
                        self.current_pv = pv;
                    }
                    alpha = alpha.max(best_score);
                } else {
                    if score < best_score {
                        best_score = score;
                        current_best_move = m;
                        self.current_pv = pv;
                    }
                    beta = beta.min(best_score);
                }

                log::debug!("Move: {}, Score: {}", m.to_string(), score);

                if alpha >= beta {
                    break;
                }
            }

            let is_forced_checkmate = best_score.abs() >= CHECKMATE_SCORE;

            let elapsed = start_time.elapsed();
            let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

            best_move = Some(current_best_move);
            self.search_depth += 1;

            output
                .send(UciOutput::Info(Info {
                    depth: self.search_depth,
                    sel_depth: self.max_depth_reached,
                    nodes: self.nodes,
                    nodes_per_second: nps,
                    time: elapsed.as_millis() as u32,
                    score: if is_forced_checkmate {
                        self.convert_mate_score(best_score, &self.current_pv)
                    } else {
                        self.convert_centipawn_score(best_score)
                    },
                    pv: self.current_pv.clone(),
                }))
                .unwrap();

            if is_forced_checkmate {
                break;
            }
        }

        best_move.unwrap()
    }
}

impl MinimaxEngine {
    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u32,
        mut alpha: f32,
        mut beta: f32,
    ) -> (f32, Vec<ChessMove>) {
        match board.status() {
            BoardStatus::Checkmate => {
                self.nodes += 1;
                if board.side_to_move() == chess::Color::White {
                    return (-CHECKMATE_SCORE * (depth as f32 + 1.0), Vec::new());
                } else {
                    return (CHECKMATE_SCORE * (depth as f32 + 1.0), Vec::new());
                }
            }
            BoardStatus::Stalemate => {
                self.nodes += 1;
                return (0.0, Vec::new());
            }
            BoardStatus::Ongoing => {}
        }

        if depth >= self.search_depth {
            return self.quiescence_search(board, alpha, beta, depth);
        }

        let mut maybe_tt_move = None;
        if let Some((tt_value, tt_bound, tt_move)) = self.probe_tt(board, depth) {
            maybe_tt_move = tt_move; // Store the move for later use
                                     // If it's an EXACT result, we can just return.
            match tt_bound {
                Bound::Exact => {
                    return (tt_value, maybe_tt_move.map_or(Vec::new(), |m| vec![m]));
                }
                Bound::Lower => {
                    // This is effectively alpha
                    if tt_value > alpha {
                        alpha = tt_value;
                    }
                }
                Bound::Upper => {
                    // This is effectively beta
                    if tt_value < beta {
                        beta = tt_value;
                    }
                }
            }
            if alpha >= beta {
                // We can do a cutoff
                return (tt_value, maybe_tt_move.map_or(Vec::new(), |m| vec![m]));
            }
        }

        let mut preferred_moves = AHashMap::with_capacity(3);

        // First priority is the current PV move
        if let Some(&move_) = self.current_pv.get(depth as usize) {
            preferred_moves.insert(move_, i32::MAX);
        }

        // Then any hash move from the tt
        if maybe_tt_move.is_some() {
            preferred_moves.insert(maybe_tt_move.unwrap(), i32::MAX - 1);
        }

        // Add killer moves for this specific depth if they are legal
        // But these are not as good as capture moves!
        for &killer_move_opt in &self.killer_moves[depth as usize] {
            if let Some(killer_move) = killer_move_opt {
                // don't need to check if legal, it will be used as mask for legal moves.
                if !preferred_moves.contains_key(&killer_move) {
                    preferred_moves.insert(killer_move, CAPTURE_SCORE - 2);
                }
            }
        }

        // Proceed with normal alpha-beta:
        let moves = get_ordered_moves(board, Some(&preferred_moves));

        let mut best_line = Vec::new();

        if board.side_to_move() == chess::Color::White {
            let mut best_value = f32::NEG_INFINITY;
            let mut best_move = None;
            for (m, _) in moves {
                let new_board = board.make_move_new(m);
                let (value, mut line) = self.alpha_beta(&new_board, depth + 1, alpha, beta);
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

            self.store_tt(board, depth, best_value, alpha, beta, best_move);
            (best_value, best_line)
        } else {
            let mut best_value = f32::INFINITY;
            let mut best_move = None;
            for (m, _) in moves {
                let new_board = board.make_move_new(m);
                let (value, mut line) = self.alpha_beta(&new_board, depth + 1, alpha, beta);
                if value < best_value {
                    best_value = value;
                    best_move = Some(m);
                    line.insert(0, m);
                    best_line = line;
                }

                beta = beta.min(best_value);
                if beta <= alpha {
                    if let Some(m) = best_move {
                        if !board.piece_on(m.get_dest()).is_some() {
                            self.add_killer_move(depth as usize, m);
                        }
                    }

                    break;
                }
            }

            self.store_tt(board, depth, best_value, alpha, beta, best_move);
            (best_value, best_line)
        }
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: f32,
        mut beta: f32,
        depth: u32,
    ) -> (f32, Vec<ChessMove>) {
        self.nodes += 1;
        self.max_depth_reached = depth.max(self.max_depth_reached);

        // Prune if visited in some other QS search
        let board_hash = board.get_hash();
        if let Some(&score) = self.qs_tt.get(&board_hash) {
            return (score, Vec::new());
        }

        // Evaluate the board right away
        let stand_pat = evaluate_board(board);

        if stand_pat >= beta {
            return (stand_pat, Vec::new());
        }
        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let moves = get_ordered_moves(board, None);

        let moves_to_search: Vec<(ChessMove, i32)> = moves
            .into_iter()
            .filter(|&(mv, score)| {
                // We definitely want to search promotion moves and checks
                if score >= PROMOTION_SCORE || score == CHECK_SCORE {
                    return true;
                }

                // We also want to search capture moves if they are good enough
                if score >= CAPTURE_SCORE {
                    return see_naive(board, mv) >= 0.0;
                }

                false
            })
            .collect();

        if moves_to_search.is_empty() {
            return (stand_pat, Vec::new());
        }

        let maximizing = board.side_to_move() == chess::Color::White;

        let mut best_line = Vec::new();
        let mut best_eval = stand_pat;

        // For each forcing move, see if it improves things
        for (m, _) in moves_to_search {
            let new_board = board.make_move_new(m);

            // Recursively call quiescence
            let (score, mut line) = self.quiescence_search(&new_board, alpha, beta, depth + 1);
            line.insert(0, m);

            if maximizing {
                if score > best_eval {
                    best_eval = score;
                    best_line = line;
                }
                alpha = alpha.max(best_eval);
            } else {
                if score < best_eval {
                    best_eval = score;
                    best_line = line;
                }
                beta = beta.min(best_eval);
            }

            if alpha >= beta {
                break;
            }
        }

        self.qs_tt.insert(board_hash, best_eval);
        (best_eval, best_line)
    }

    #[inline]
    fn probe_tt(&mut self, board: &Board, depth: u32) -> Option<(f32, Bound, Option<ChessMove>)> {
        let board_hash = board.get_hash();
        let plies = self.search_depth - depth;

        if let Some(entry) = self.tt.get(&board_hash) {
            if entry.plies >= plies {
                return Some((entry.value, entry.bound, entry.best_move));
            }
        }
        None
    }

    fn store_tt(
        &mut self,
        board: &Board,
        depth: u32,
        value: f32,
        alpha: f32,
        beta: f32,
        best_move: Option<ChessMove>,
    ) {
        let plies = self.search_depth - depth;

        let bound = if value <= alpha {
            Bound::Upper
        } else if value >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };

        let board_hash = board.get_hash();
        let entry = TTEntry {
            plies,
            value,
            bound,
            best_move,
        };

        if let Some(old_entry) = self.tt.get(&board_hash) {
            if old_entry.plies <= plies {
                self.tt.insert(board_hash, entry);
            }
        } else {
            self.tt.insert(board_hash, entry);
        }
    }

    #[inline]
    fn add_killer_move(&mut self, depth: usize, m: ChessMove) {
        let killers = &mut self.killer_moves[depth];
        if killers[0] != Some(m) {
            killers[1] = killers[0];
            killers[0] = Some(m);
        }
    }

    fn convert_mate_score(&self, score: f32, pv: &Vec<ChessMove>) -> Score {
        let is_winning = (score > 0.0) == (self.board.side_to_move() == chess::Color::White);
        let mate_in = if is_winning {
            pv.len() as i32 - 1
        } else {
            -((pv.len() as i32) - 1)
        };
        Score::Mate(mate_in)
    }

    fn convert_centipawn_score(&self, score: f32) -> Score {
        let cp_score = if self.board.side_to_move() == chess::Color::White {
            score as i32
        } else {
            -(score as i32)
        };
        Score::Centipawns(cp_score)
    }
}

#[inline]
fn see_naive(board: &Board, capture_move: ChessMove) -> f32 {
    if let (Some(captured_piece), Some(capturing_piece)) = (
        board.piece_on(capture_move.get_dest()),
        board.piece_on(capture_move.get_source()),
    ) {
        piece_value(captured_piece) - piece_value(capturing_piece)
    } else {
        0.0
    }
}
