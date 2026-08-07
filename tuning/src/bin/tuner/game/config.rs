/// Configuration for a game.
#[derive(Clone, Copy)]
pub struct GameConfig {
    pub nodes: u64,
    pub max_plies: u64,

    // Adjudication
    pub resign_score: i16,
    pub resign_moves: u64,
    pub draw_score: i16,
    pub draw_moves: u64,
    pub draw_after: u64,
}
