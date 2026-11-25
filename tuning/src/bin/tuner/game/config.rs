use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Nodes { per_move: u64 },
    Time { initial: Duration },
}

/// Configuration for a game.
#[derive(Clone, Copy)]
pub struct GameConfig {
    pub search_mode: SearchMode,
    pub max_plies: u64,

    // Adjudication
    pub resign_score: i16,
    pub resign_moves: u64,
    pub draw_score: i16,
    pub draw_moves: u64,
    pub draw_after: u64,
}
