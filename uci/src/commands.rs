use ahash::AHashSet;
use cozy_chess::Board;

/// Commands received from the UCI GUI.
#[derive(Debug)]
pub enum UciInput {
    Uci,
    IsReady,

    UciNewGame,
    Position {
        board: Board,
        game_history: AHashSet<u64>,
    },
    Go(GoParams),

    Stop,
    Quit,
    SetOption {
        name: String,
        value: String,
    },
    Unknown(String),
}

/// Commands sent to the UCI GUI.
#[derive(Debug)]
pub enum UciOutput {
    IdName(String),
    IdAuthor(String),
    UciOk,
    ReadyOk,
    BestMove(String),
    Info(Info),
    Option(String),
    Raw(String),
}

/// Search information sent to the GUI during analysis.
///
/// Example: `info depth 4 seldepth 7 nodes 3274 nps 922805 time 3 score cp 10 pv e2e4 d7d5 e4d5 d8d5`
#[derive(Debug, Default)]
pub struct Info {
    pub depth: u8,
    pub sel_depth: u8,
    pub nodes: u32,
    pub nodes_per_second: u32,
    pub time: u32,
    pub pv: Vec<String>,
    pub score: Score,
}

/// Evaluation score in UCI format.
#[derive(Debug)]
pub enum Score {
    /// Score in centipawns.
    Centipawns(i16),
    /// Mate in N moves. Positive = we mate, negative = we get mated.
    Mate(i16),
}

impl Default for Score {
    fn default() -> Self {
        Score::Centipawns(0)
    }
}

/// Parameters for the "go" command.
#[derive(Debug, Default)]
pub struct GoParams {
    /// Search until "stop" is received.
    pub infinite: bool,
    /// Restrict search to these moves only. TODO: not yet implemented.
    pub search_moves: Option<Vec<String>>,
    /// White's remaining time in milliseconds.
    pub wtime: Option<u64>,
    /// Black's remaining time in milliseconds.
    pub btime: Option<u64>,
    /// White's increment per move in milliseconds.
    pub winc: Option<u64>,
    /// Black's increment per move in milliseconds.
    pub binc: Option<u64>,
    /// Moves until next time control (sudden death if not set).
    pub moves_to_go: Option<u64>,
    /// Search to this depth only.
    pub depth: Option<u8>,
    /// Search for exactly this many milliseconds.
    pub move_time: Option<u64>,
}

#[derive(Debug, Default)]
pub struct EngineOptions {}
