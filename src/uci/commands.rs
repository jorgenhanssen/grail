use chess::{Board, ChessMove};

#[derive(Debug)]
pub enum UciInput {
    Uci,
    IsReady,

    UciNewGame,
    Position(Board),
    Go(GoParams),

    Stop,
    Quit,
    Unknown(String),
    // TODO: Implement
    // SetOption { name: String, value: Option<String> },
    // PonderHit,
}

#[derive(Debug)]
pub enum UciOutput {
    IdName(String),
    IdAuthor(String),
    UciOk,
    ReadyOk,
    BestMove {
        bestmove: ChessMove,
        ponder: Option<ChessMove>,
    },
    Info(Info),
    // TODO: Implement
    // EngineOptions(EngineOptions),
}

#[derive(Debug)]
pub struct Info {
    pub depth: u8,
    pub nodes: u64,
    pub nodes_per_second: u64,
    pub time: u64,
    pub line: Vec<ChessMove>,
    pub score: i32, // centipawns
}

#[derive(Debug)]
pub struct GoParams {
    // Bool to enable pondering mode. The engine will not stop pondering in the background until a stop command is received.
    pub ponder: bool,

    // Search in the background until a stop command is received.
    pub infinite: bool,

    // Restrict search to moves in this list.
    pub searchmoves: Option<Vec<String>>,

    // Integer of milliseconds White has left on the clock.
    pub wtime: Option<u64>,

    // Integer of milliseconds Black has left on the clock.
    pub btime: Option<u64>,

    // Integer of white Fisher increment.
    pub winc: Option<u64>,

    // Integer of black Fisher increment.
    pub binc: Option<u64>,

    // Number of moves to the next time control. If this is not set, but wtime or btime are, then it is sudden death.
    pub movestogo: Option<u64>,

    // Search depth ply only.
    pub depth: Option<u64>,

    // Search so many nodes only.
    pub nodes: Option<u64>,

    // Search exactly movetime milliseconds.
    pub movetime: Option<u64>,
}

#[derive(Debug, Default)]
pub struct EngineOptions {}
