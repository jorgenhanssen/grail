use cozy_chess::{BitBoard, Board, Color, Piece, Square};
use utils::bitset::Bitset;

// Feature layout (total 1153 features):
// - Piece placements: 64 squares × 6 pieces × 2 colors = 768 features
// - Support: squares where each color defends own pieces = 128 features (64 per color)
// - Space: squares each color controls (non-piece squares) = 128 features
// - Threats: squares where each color threatens enemy pieces = 128 features
// - Side to move: 1 feature (1 if White to move)
const NUM_PIECE_PLACEMENT_FEATURES: usize = Square::NUM * Piece::NUM * Color::NUM;
const NUM_SUPPORT_FEATURES: usize = Square::NUM * 2;
const NUM_SPACE_FEATURES: usize = Square::NUM * 2;
const NUM_THREAT_FEATURES: usize = Square::NUM * 2;
const NUM_SIDE_TO_MOVE_FEATURES: usize = 1;

pub const NUM_FEATURES: usize = NUM_PIECE_PLACEMENT_FEATURES
    + NUM_SUPPORT_FEATURES
    + NUM_SPACE_FEATURES
    + NUM_THREAT_FEATURES
    + NUM_SIDE_TO_MOVE_FEATURES; // 1153 total

const PIECE_FEATURES_END: usize = NUM_PIECE_PLACEMENT_FEATURES;
const WHITE_SUPPORT_START: usize = PIECE_FEATURES_END;
const WHITE_SUPPORT_END: usize = WHITE_SUPPORT_START + Square::NUM;
const BLACK_SUPPORT_START: usize = WHITE_SUPPORT_END;
const BLACK_SUPPORT_END: usize = BLACK_SUPPORT_START + Square::NUM;
const WHITE_SPACE_START: usize = BLACK_SUPPORT_END;
const WHITE_SPACE_END: usize = WHITE_SPACE_START + Square::NUM;
const BLACK_SPACE_START: usize = WHITE_SPACE_END;
const BLACK_SPACE_END: usize = BLACK_SPACE_START + Square::NUM;
const WHITE_THREATS_START: usize = BLACK_SPACE_END;
const WHITE_THREATS_END: usize = WHITE_THREATS_START + Square::NUM;
const BLACK_THREATS_START: usize = WHITE_THREATS_END;
#[allow(dead_code)]
const BLACK_THREATS_END: usize = BLACK_THREATS_START + Square::NUM;
const SIDE_TO_MOVE_IDX: usize = NUM_FEATURES - 1;

/// Encodes a board position into a dense f32 feature array.
/// Used during training where f32 tensors are required.
pub fn encode_board(
    board: &Board,
    white_attacks: BitBoard,
    black_attacks: BitBoard,
    white_support: BitBoard,
    black_support: BitBoard,
    white_threats: BitBoard,
    black_threats: BitBoard,
) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    // Piece placements
    for sq in Square::ALL {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let offset = sq as usize * 12 + piece_color_to_index(piece, color);
            features[offset] = 1.0;
        }
    }

    // White support
    for sq in white_support {
        features[WHITE_SUPPORT_START + sq as usize] = 1.0;
    }

    // Black support
    for sq in black_support {
        features[BLACK_SUPPORT_START + sq as usize] = 1.0;
    }

    // White space (controlled non-piece squares)
    let white_pieces = board.colors(Color::White);
    let white_space_bb = white_attacks & !white_pieces;
    for sq in white_space_bb {
        features[WHITE_SPACE_START + sq as usize] = 1.0;
    }

    // Black space
    let black_pieces = board.colors(Color::Black);
    let black_space_bb = black_attacks & !black_pieces;
    for sq in black_space_bb {
        features[BLACK_SPACE_START + sq as usize] = 1.0;
    }

    // White threats
    for sq in white_threats {
        features[WHITE_THREATS_START + sq as usize] = 1.0;
    }

    // Black threats
    for sq in black_threats {
        features[BLACK_THREATS_START + sq as usize] = 1.0;
    }

    // Side to move
    if board.side_to_move() == Color::White {
        features[SIDE_TO_MOVE_IDX] = 1.0;
    }

    features
}

/// Encodes a board position into a packed bitset for inference.
///
/// Bitset is faster than f32 for inference: XOR finds changed features instantly,
/// and storage is 64x denser (64 bits per u64 vs one f32 per feature).
/// Training still uses the f32 version above since tensors require floats.
pub fn encode_board_bitset(
    board: &Board,
    white_attacks: BitBoard,
    black_attacks: BitBoard,
    white_support: BitBoard,
    black_support: BitBoard,
    white_threats: BitBoard,
    black_threats: BitBoard,
) -> Bitset<NUM_FEATURES> {
    let mut bitset = Bitset::default();

    // Piece placements
    for sq in Square::ALL {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let idx = sq as usize * 12 + piece_color_to_index(piece, color);
            bitset.set(idx);
        }
    }

    // White support
    for sq in white_support {
        bitset.set(WHITE_SUPPORT_START + sq as usize);
    }

    // Black support
    for sq in black_support {
        bitset.set(BLACK_SUPPORT_START + sq as usize);
    }

    // White space (controlled non-piece squares)
    let white_pieces = board.colors(Color::White);
    let white_space_bb = white_attacks & !white_pieces;
    for sq in white_space_bb {
        bitset.set(WHITE_SPACE_START + sq as usize);
    }

    // Black space
    let black_pieces = board.colors(Color::Black);
    let black_space_bb = black_attacks & !black_pieces;
    for sq in black_space_bb {
        bitset.set(BLACK_SPACE_START + sq as usize);
    }

    // White threats
    for sq in white_threats {
        bitset.set(WHITE_THREATS_START + sq as usize);
    }

    // Black threats
    for sq in black_threats {
        bitset.set(BLACK_THREATS_START + sq as usize);
    }

    // Side to move
    if board.side_to_move() == Color::White {
        bitset.set(SIDE_TO_MOVE_IDX);
    }

    bitset
}

fn piece_color_to_index(piece: Piece, color: Color) -> usize {
    match (color, piece) {
        (Color::White, Piece::Pawn) => 0,
        (Color::White, Piece::Knight) => 1,
        (Color::White, Piece::Bishop) => 2,
        (Color::White, Piece::Rook) => 3,
        (Color::White, Piece::Queen) => 4,
        (Color::White, Piece::King) => 5,

        (Color::Black, Piece::Pawn) => 6,
        (Color::Black, Piece::Knight) => 7,
        (Color::Black, Piece::Bishop) => 8,
        (Color::Black, Piece::Rook) => 9,
        (Color::Black, Piece::Queen) => 10,
        (Color::Black, Piece::King) => 11,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::board_metrics::BoardMetrics;

    const TEST_POSITIONS: &[&str] = &[
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", // Starting
        "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2", // After 1.e4 Nc6
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4", // Italian
        "rnbqkb1r/pp1p1ppp/4pn2/2p5/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4", // Sicilian
        "8/8/8/8/8/5k2/8/4K2R w - - 0 1",                           // Endgame
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", // Kiwipete
    ];

    #[test]
    fn test_encode_board_and_bitset_are_consistent() {
        for fen in TEST_POSITIONS {
            let board: Board = fen.parse().unwrap();
            let metrics = BoardMetrics::new(&board);

            let features = encode_board(
                &board,
                metrics.attacks[Color::White as usize],
                metrics.attacks[Color::Black as usize],
                metrics.support[Color::White as usize],
                metrics.support[Color::Black as usize],
                metrics.threats[Color::White as usize],
                metrics.threats[Color::Black as usize],
            );

            let bitset = encode_board_bitset(
                &board,
                metrics.attacks[Color::White as usize],
                metrics.attacks[Color::Black as usize],
                metrics.support[Color::White as usize],
                metrics.support[Color::Black as usize],
                metrics.threats[Color::White as usize],
                metrics.threats[Color::Black as usize],
            );

            for (i, &f) in features.iter().enumerate() {
                assert_eq!(
                    f == 1.0,
                    bitset.get(i),
                    "Mismatch at feature {} for FEN: {}",
                    i,
                    fen
                );
            }
        }
    }
}
