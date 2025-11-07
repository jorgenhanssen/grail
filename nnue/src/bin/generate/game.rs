use chess::{Board, Game};
use utils::has_insufficient_material;

pub fn game_is_terminal(
    game: &Game,
    board: &Board,
    position_counts: &mut std::collections::HashMap<u64, usize>,
) -> bool {
    // 1. Check chess rules (checkmate, stalemate, draw acceptance, etc.)
    if game.result().is_some() {
        return true;
    }

    // 2. Check insufficient material (K vs K, K+B vs K, K+N vs K, etc.)
    if has_insufficient_material(board) {
        return true;
    }

    // 3. Check position repetition (abort on first repetition)
    // For training data, we don't need official three-fold rule -
    // any repetition means the game is cycling and won't produce useful data
    let board_hash = board.get_hash();
    *position_counts.entry(board_hash).or_insert(0) += 1;
    if position_counts[&board_hash] >= 2 {
        return true;
    }

    false
}
