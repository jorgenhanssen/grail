use std::sync::mpsc::Sender;

use crate::{
    uci::{
        commands::{GoParams, Info},
        UciOutput,
    },
    utils::evaluate_board,
};
use chess::{Board, BoardStatus, ChessMove, MoveGen, Piece};

pub struct Engine {
    board: Board,
    nodes: u32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::default(),
            nodes: 0,
        }
    }

    pub fn set_position(&mut self, board: Board) {
        self.board = board;
    }

    pub fn search(&mut self, params: &GoParams, output: &Sender<UciOutput>) -> ChessMove {
        self.nodes = 0;
        let search_time = params.move_time.unwrap_or(10_000);
        let start_time = std::time::Instant::now();

        let mut current_depth: u8 = 1;
        let mut best_move = None;

        while start_time.elapsed().as_millis() < search_time as u128 {
            let mut alpha = f32::NEG_INFINITY;
            let mut beta = f32::INFINITY;
            let moves_with_scores = get_ordered_moves(&self.board);

            let maximizing = self.board.side_to_move() == chess::Color::White;
            let mut best_score = if maximizing {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
            let mut current_best_move = moves_with_scores[0];
            let mut best_line = Vec::new();

            for m in moves_with_scores {
                let new_board = self.board.make_move_new(m);
                let (score, mut line) = self.alpha_beta(&new_board, current_depth - 1, alpha, beta);
                line.insert(0, m); // Add current move to the beginning of the line

                if maximizing {
                    if score > best_score {
                        best_score = score;
                        current_best_move = m;
                        best_line = line;
                    }
                    alpha = alpha.max(best_score);
                } else {
                    if score < best_score {
                        best_score = score;
                        current_best_move = m;
                        best_line = line;
                    }
                    beta = beta.min(best_score);
                }

                // log current move and it's score
                log::debug!("Move: {}, Score: {}", m.to_string(), score);

                if alpha >= beta {
                    break;
                }
            }

            let elapsed = start_time.elapsed();
            let nps = (self.nodes as f32 / elapsed.as_secs_f32()) as u32;

            output
                .send(UciOutput::Info(Info {
                    depth: current_depth,
                    nodes: self.nodes,
                    nodes_per_second: nps,
                    time: elapsed.as_millis() as u32,
                    line: best_line, // Use the best line we found
                    score: best_score as i32,
                }))
                .unwrap();

            best_move = Some(current_best_move);
            current_depth += 1;
        }

        best_move.unwrap()
    }

    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: f32,
        mut beta: f32,
    ) -> (f32, Vec<ChessMove>) {
        match board.status() {
            BoardStatus::Checkmate => {
                self.nodes += 1;
                if board.side_to_move() == chess::Color::White {
                    return (-10_000.0 * (depth as f32 + 1.0), Vec::new());
                } else {
                    return (10_000.0 * (depth as f32 + 1.0), Vec::new());
                }
            }
            BoardStatus::Stalemate => {
                self.nodes += 1;
                return (0.0, Vec::new());
            }
            BoardStatus::Ongoing => { /* continue */ }
        }

        if depth == 0 {
            self.nodes += 1;
            return (evaluate_board(board), Vec::new());
        }

        let moves = get_ordered_moves(board);
        let mut best_line = Vec::new();

        if board.side_to_move() == chess::Color::White {
            let mut best_value = f32::NEG_INFINITY;
            for m in moves {
                let new_board = board.make_move_new(m);
                let (value, mut line) = self.alpha_beta(&new_board, depth - 1, alpha, beta);
                if value > best_value {
                    best_value = value;
                    line.insert(0, m);
                    best_line = line;
                }
                alpha = alpha.max(best_value);

                if alpha >= beta {
                    break;
                }
            }
            (best_value, best_line)
        } else {
            let mut best_value = f32::INFINITY;
            for m in moves {
                let new_board = board.make_move_new(m);
                let (value, mut line) = self.alpha_beta(&new_board, depth - 1, alpha, beta);
                if value < best_value {
                    best_value = value;
                    line.insert(0, m);
                    best_line = line;
                }
                beta = beta.min(best_value);

                if beta <= alpha {
                    break;
                }
            }
            (best_value, best_line)
        }
    }
}

fn get_ordered_moves(board: &Board) -> Vec<ChessMove> {
    // Pre-allocate vector for moves and their scores
    let mut moves_with_scores: Vec<(ChessMove, i32)> = MoveGen::new_legal(board)
        .map(|m| (m, mvv_lva(m, board)))
        .collect();

    // Sort by score (descending)
    moves_with_scores.sort_unstable_by_key(|&(_, score)| -score);

    // Convert to moves vector, reusing the allocation
    moves_with_scores.into_iter().map(|(m, _)| m).collect()
}

fn mvv_lva(move_: ChessMove, board: &Board) -> i32 {
    // Check for promotions first - these are usually very good moves
    if let Some(promotion) = move_.get_promotion() {
        return match promotion {
            Piece::Queen => 20000,
            Piece::Rook => 19000,
            Piece::Bishop | Piece::Knight => 18000,
            _ => 0,
        };
    }

    let resulting_board = board.make_move_new(move_);

    // Checks are very important but slightly below promotions
    if resulting_board.checkers().popcnt() > 0 {
        return 15000;
    }

    // Next look at captures
    if let Some(victim) = board.piece_on(move_.get_dest()) {
        let victim_value = match victim {
            Piece::Queen => 900,
            Piece::Rook => 500,
            Piece::Bishop => 330, // Slightly higher than knight
            Piece::Knight => 320,
            Piece::Pawn => 100,
            Piece::King => 0, // Shouldn't happen in legal moves
        };

        let attacker = board.piece_on(move_.get_source()).unwrap();
        let attacker_value = match attacker {
            Piece::Queen => 900,
            Piece::Rook => 500,
            Piece::Bishop => 330,
            Piece::Knight => 320,
            Piece::Pawn => 100,
            Piece::King => 0,
        };

        // Score = 10 * victim value - attacker value
        // This ensures capturing a queen with a pawn is better than capturing a pawn with a queen
        return victim_value * 10 - attacker_value;
    }

    // For non-capture moves, return a small positive value based on the piece type
    // This encourages moving more valuable pieces first in quiet positions
    let piece = board.piece_on(move_.get_source()).unwrap();
    match piece {
        Piece::Queen => 50,
        Piece::Rook => 40,
        Piece::Bishop => 30,
        Piece::Knight => 30,
        Piece::Pawn => 20,
        Piece::King => 10,
    }
}
