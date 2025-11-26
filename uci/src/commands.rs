use ahash::AHashSet;
use cozy_chess::Board;

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

#[derive(Debug)]
pub enum Score {
    Centipawns(i16), // centipawns
    Mate(i16),       // Positive for mate-in-n, negative for mated-in-n
}

impl Default for Score {
    fn default() -> Self {
        Score::Centipawns(0)
    }
}

#[derive(Debug, Default)]
pub struct GoParams {
    // Search in the background until a stop command is received.
    pub infinite: bool,

    // Restrict search to moves in this list.
    pub search_moves: Option<Vec<String>>,

    // Integer of milliseconds White has left on the clock.
    pub wtime: Option<u64>,

    // Integer of milliseconds Black has left on the clock.
    pub btime: Option<u64>,

    // Integer of white Fisher increment.
    pub winc: Option<u64>,

    // Integer of black Fisher increment.
    pub binc: Option<u64>,

    // Number of moves to the next time control. If this is not set, but wtime or btime are, then it is sudden death.
    pub moves_to_go: Option<u64>,

    // Search depth ply only.
    pub depth: Option<u8>,

    // Search exactly movetime milliseconds.
    pub move_time: Option<u64>,
}

#[derive(Debug, Default)]
pub struct EngineOptions {}
