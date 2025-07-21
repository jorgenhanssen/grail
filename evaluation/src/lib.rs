pub mod def;
pub mod piece_values;
pub mod scores;
pub mod traditional;

pub use def::Evaluator;
pub use piece_values::{piece_value, total_material};
pub use piece_values::{
    BISHOP_VALUE, KING_VALUE, KNIGHT_VALUE, PAWN_VALUE, QUEEN_VALUE, ROOK_VALUE,
};
pub use traditional::TraditionalEvaluator;
