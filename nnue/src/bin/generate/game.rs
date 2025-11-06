use chess::{BitBoard, Board, Color, Game, GameResult, Piece};
use std::sync::atomic::Ordering;

const MATE_THRESHOLD: i16 = 5000;
const STABLE_DRAW_MOVES: usize = 40;
const DRAWISH_EVAL: i16 = 20;
const BALANCED_POSITION_THRESHOLD: i16 = 1000;

const LIGHT_SQUARES_MASK: u64 = 0x55AA55AA55AA55AA;

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

pub fn has_insufficient_material(board: &Board) -> bool {
    let pawns = board.pieces(Piece::Pawn);
    let rooks = board.pieces(Piece::Rook);
    let queens = board.pieces(Piece::Queen);
    if (pawns | rooks | queens).popcnt() > 0 {
        return false;
    }

    // Only kings and minor pieces remain
    let white = board.color_combined(Color::White);
    let black = board.color_combined(Color::Black);
    let knights = board.pieces(Piece::Knight);
    let bishops = board.pieces(Piece::Bishop);

    let white_knights = (white & knights).popcnt();
    let black_knights = (black & knights).popcnt();
    let white_bishops = (white & bishops).popcnt();
    let black_bishops = (black & bishops).popcnt();

    let white_minors = white_knights + white_bishops;
    let black_minors = black_knights + black_bishops;

    // K vs K
    if white_minors == 0 && black_minors == 0 {
        return true;
    }

    // K+N vs K or K vs K+N
    if white_minors == 1 && white_knights == 1 && black_minors == 0 {
        return true;
    }
    if black_minors == 1 && black_knights == 1 && white_minors == 0 {
        return true;
    }

    // K+B vs K or K vs K+B
    if white_minors == 1 && white_bishops == 1 && black_minors == 0 {
        return true;
    }
    if black_minors == 1 && black_bishops == 1 && white_minors == 0 {
        return true;
    }

    // K+B vs K+B with bishops on same color squares
    if white_bishops == 1 && black_bishops == 1 && white_minors == 1 && black_minors == 1 {
        let light_squares = BitBoard(LIGHT_SQUARES_MASK);
        let white_on_light = (white & bishops & light_squares).popcnt() > 0;
        let black_on_light = (black & bishops & light_squares).popcnt() > 0;

        // Both on light or both on dark = insufficient material
        if white_on_light == black_on_light {
            return true;
        }
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

pub fn is_decisive_game(game: &Game, game_end_reason: &Option<GameEndReason>) -> bool {
    if let Some(result) = game.result() {
        return match result {
            GameResult::WhiteCheckmates | GameResult::BlackResigns => true,
            GameResult::BlackCheckmates | GameResult::WhiteResigns => true,
            GameResult::Stalemate | GameResult::DrawAccepted | GameResult::DrawDeclared => false,
        };
    }

    // Game was aborted early - check why
    match game_end_reason {
        Some(GameEndReason::ChessRules) => {
            // Should have been caught above, but handle it anyway
            false
        }
        Some(GameEndReason::MateScore) => {
            // Game aborted due to mate score - it's a decisive game
            true
        }
        Some(GameEndReason::InsufficientMaterial)
        | Some(GameEndReason::Repetition)
        | Some(GameEndReason::StableDraw)
        | None => {
            // Drawn games
            false
        }
    }
}

pub fn flush_game_to_evaluations(
    game: &Game,
    game_id: usize,
    game_end_reason: &Option<GameEndReason>,
    current_game_positions: &mut Vec<(String, i16)>,
    evaluations: &mut Vec<(String, i16, usize)>,
    histogram: &crate::histogram::HistogramHandle,
    sample_counter: &std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let is_decisive = is_decisive_game(game, game_end_reason);

    let (positions, scores): (Vec<_>, Vec<_>) = if is_decisive {
        // Include all positions in decisive games
        current_game_positions
            .drain(..)
            .map(|(fen, score)| ((fen, score, game_id), score))
            .unzip()
    } else {
        // Only include balanced positions in drawn games to prevent
        // labeling clearly winning positions as draws
        current_game_positions
            .drain(..)
            .filter(|(_, score)| score.abs() < BALANCED_POSITION_THRESHOLD)
            .map(|(fen, score)| ((fen, score, game_id), score))
            .unzip()
    };

    let num_positions = positions.len();
    evaluations.extend(positions);

    // Update histogram and sample counter
    histogram.record_scores(&scores);
    sample_counter.fetch_add(num_positions, Ordering::Relaxed);
}
