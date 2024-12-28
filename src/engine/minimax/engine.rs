use crate::engine::Engine;
use crate::utils::get_ordered_moves;
use crate::{
    uci::{
        commands::{GoParams, Info},
        UciOutput,
    },
    utils::evaluate_board,
};
use chess::{Board, BoardStatus, ChessMove};
use std::sync::mpsc::Sender;

pub struct MinimaxEngine {
    board: Board,
    nodes: u32,
}

impl Default for MinimaxEngine {
    fn default() -> Self {
        Self {
            board: Board::default(),
            nodes: 0,
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
                    line: best_line,
                    score: best_score as i32,
                }))
                .unwrap();

            best_move = Some(current_best_move);
            current_depth += 1;
        }

        best_move.unwrap()
    }
}

impl MinimaxEngine {
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
            BoardStatus::Ongoing => {}
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
