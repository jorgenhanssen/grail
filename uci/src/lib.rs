mod connection;
mod decoder;
mod encoder;
mod options;
mod utils;

pub mod commands;

pub use commands::{UciInput, UciOutput};
pub use connection::UciConnection;
pub use decoder::Decoder;
pub use encoder::Encoder;
pub use options::{UciOption, UciOptionType};
pub use utils::{move_to_uci, pv_to_uci};

/// Null move in UCI format, used when no legal move exists (e.g., checkmate).
/// Per UCI spec, this should be sent as the bestmove when the position has no legal moves.
pub const NULL_MOVE: &str = "0000";
