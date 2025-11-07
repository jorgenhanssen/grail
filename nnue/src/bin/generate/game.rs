use chess::{Board, Game};
use std::sync::atomic::Ordering;
use utils::has_insufficient_material;

const MATE_THRESHOLD: i16 = 5000;
const STABLE_DRAW_MOVES: usize = 40;
const DRAWISH_EVAL: i16 = 20;

pub enum GameEndReason {
    ChessRules,           // Checkmate, stalemate, etc.
    MateScore,            // Mate detected by evaluation
    InsufficientMaterial, // K vs K, K+B vs K, etc.
    Repetition,           // Three-fold repetition
    StableDraw,           // 40+ moves with eval < 20cp
}

pub fn check_draw(
    game: &Game,
    board: &Board,
    position_counts: &mut std::collections::HashMap<u64, usize>,
    game_end_reason: &mut Option<GameEndReason>,
) -> bool {
    // 1. Check chess rules (checkmate, stalemate, draw acceptance, etc.)
    if let Some(_result) = game.result() {
        *game_end_reason = Some(GameEndReason::ChessRules);
        return true;
    }

    // 2. Check insufficient material (K vs K, K+B vs K, K+N vs K, etc.)
    if has_insufficient_material(board) {
        *game_end_reason = Some(GameEndReason::InsufficientMaterial);
        return true;
    }

    // 3. Check position repetition (abort on first repetition)
    // For training data, we don't need official three-fold rule -
    // any repetition means the game is cycling and won't produce useful data
    let board_hash = board.get_hash();
    *position_counts.entry(board_hash).or_insert(0) += 1;
    if position_counts[&board_hash] >= 2 {
        *game_end_reason = Some(GameEndReason::Repetition);
        return true;
    }

    false
}

pub fn should_abort_game(
    score: &i16,
    current_game_positions: &[(String, i16)],
    game_end_reason: &mut Option<GameEndReason>,
) -> bool {
    // Less good for the model to train on squashed mate scores
    if score.abs() >= MATE_THRESHOLD {
        *game_end_reason = Some(GameEndReason::MateScore);
        return true;
    }

    let num_moves: usize = current_game_positions.len();

    // Abort if evaluation has been stable and near-zero for a while
    if num_moves >= STABLE_DRAW_MOVES {
        let start_idx = num_moves - STABLE_DRAW_MOVES;
        let last_positions = &current_game_positions[start_idx..];

        let all_drawish = last_positions
            .iter()
            .all(|(_, eval)| eval.abs() < DRAWISH_EVAL);

        if all_drawish {
            // Game has been positionally balanced for STABLE_DRAW_MOVES+ moves - likely a dead draw
            *game_end_reason = Some(GameEndReason::StableDraw);
            return true;
        }
    }

    false
}

pub fn flush_game_to_evaluations(
    game_id: usize,
    current_game_positions: &mut Vec<(String, i16)>,
    evaluations: &mut Vec<(String, i16, usize)>,
    histogram: &crate::histogram::HistogramHandle,
    sample_counter: &std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let (positions, scores): (Vec<_>, Vec<_>) = current_game_positions
        .drain(..)
        .map(|(fen, score)| ((fen, score, game_id), score))
        .unzip();

    let num_positions = positions.len();
    evaluations.extend(positions);

    // Update histogram and sample counter
    histogram.record_scores(&scores);
    sample_counter.fetch_add(num_positions, Ordering::Relaxed);
}
