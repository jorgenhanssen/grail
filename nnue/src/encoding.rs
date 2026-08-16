use cozy_chess::{BitBoard, Board, Color, Piece, Square};

use crate::bitset;

const NUM_PIECE_PLACEMENT_FEATURES: usize = Square::NUM * Piece::NUM * Color::NUM;
const NUM_SUPPORT_FEATURES: usize = Square::NUM * 2;
const NUM_THREAT_FEATURES: usize = Square::NUM * 2;

pub const NUM_FEATURES: usize =
    NUM_PIECE_PLACEMENT_FEATURES + NUM_SUPPORT_FEATURES + NUM_THREAT_FEATURES; // 1024

// Exported to the analysis tool
pub const PIECE_FEATURES_START: usize = 0;
pub const PIECE_FEATURES_END: usize = NUM_PIECE_PLACEMENT_FEATURES;
pub const US_SUPPORT_START: usize = PIECE_FEATURES_END;
pub const US_SUPPORT_END: usize = US_SUPPORT_START + Square::NUM;
pub const THEM_SUPPORT_START: usize = US_SUPPORT_END;
pub const THEM_SUPPORT_END: usize = THEM_SUPPORT_START + Square::NUM;
pub const US_THREATS_START: usize = THEM_SUPPORT_END;
pub const US_THREATS_END: usize = US_THREATS_START + Square::NUM;
pub const THEM_THREATS_START: usize = US_THREATS_END;
pub const THEM_THREATS_END: usize = THEM_THREATS_START + Square::NUM;

/// Encodes a board position into a dense f32 feature array from a perspective.
/// Used during training where f32 tensors are required.
pub fn encode_board(
    board: &Board,
    white_support: BitBoard,
    black_support: BitBoard,
    white_threats: BitBoard,
    black_threats: BitBoard,
    perspective: Color,
) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    // Piece placements
    for color in [Color::White, Color::Black] {
        let side_offset = if color == perspective { 0 } else { Piece::NUM };
        for piece in Piece::ALL {
            let piece_idx = side_offset + piece as usize;
            for sq in board.colored_pieces(color, piece) {
                let sq_idx = sq.relative_to(perspective) as usize;
                features[sq_idx * (Piece::NUM * Color::NUM) + piece_idx] = 1.0;
            }
        }
    }

    // Support
    let (us_support, them_support) = from_perspective(white_support, black_support, perspective);
    for sq in us_support {
        features[US_SUPPORT_START + sq as usize] = 1.0;
    }
    for sq in them_support {
        features[THEM_SUPPORT_START + sq as usize] = 1.0;
    }

    // Threats
    let (us_threats, them_threats) = from_perspective(white_threats, black_threats, perspective);
    for sq in us_threats {
        features[US_THREATS_START + sq as usize] = 1.0;
    }
    for sq in them_threats {
        features[THEM_THREATS_START + sq as usize] = 1.0;
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
    white_support: BitBoard,
    black_support: BitBoard,
    white_threats: BitBoard,
    black_threats: BitBoard,
    perspective: Color,
) -> bitset!(NUM_FEATURES) {
    let mut bitset: bitset!(NUM_FEATURES) = Default::default();

    // Piece placements
    for color in [Color::White, Color::Black] {
        let side_offset = if color == perspective { 0 } else { Piece::NUM };
        for piece in Piece::ALL {
            let piece_idx = side_offset + piece as usize;
            for sq in board.colored_pieces(color, piece) {
                let sq_idx = sq.relative_to(perspective) as usize;
                bitset.set(sq_idx * (Piece::NUM * Color::NUM) + piece_idx);
            }
        }
    }

    // The bitboard sections all start at 64-bit aligned offsets, so let's write
    // whole ranks at once instead of iterating per square.

    // Support
    let (us_support, them_support) = from_perspective(white_support, black_support, perspective);
    bitset.set_u64(bitset.u64_index(US_SUPPORT_START), us_support.0);
    bitset.set_u64(bitset.u64_index(THEM_SUPPORT_START), them_support.0);

    // Threats
    let (us_threats, them_threats) = from_perspective(white_threats, black_threats, perspective);
    bitset.set_u64(bitset.u64_index(US_THREATS_START), us_threats.0);
    bitset.set_u64(bitset.u64_index(THEM_THREATS_START), them_threats.0);

    bitset
}

/// (us, them) bitboards relative to perspective. Ranks are flipped for black
/// so that rank 1 is always our back rank.
fn from_perspective(white: BitBoard, black: BitBoard, perspective: Color) -> (BitBoard, BitBoard) {
    match perspective {
        Color::White => (white, black),
        Color::Black => (black.flip_ranks(), white.flip_ranks()),
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

            for perspective in [Color::White, Color::Black] {
                let features = encode_board(
                    &board,
                    metrics.support[Color::White as usize],
                    metrics.support[Color::Black as usize],
                    metrics.threats[Color::White as usize],
                    metrics.threats[Color::Black as usize],
                    perspective,
                );

                let bitset = encode_board_bitset(
                    &board,
                    metrics.support[Color::White as usize],
                    metrics.support[Color::Black as usize],
                    metrics.threats[Color::White as usize],
                    metrics.threats[Color::Black as usize],
                    perspective,
                );

                for (i, &f) in features.iter().enumerate() {
                    assert_eq!(
                        f == 1.0,
                        bitset.get(i),
                        "Mismatch at feature {} for FEN: {} ({:?})",
                        i,
                        fen,
                        perspective
                    );
                }
            }
        }
    }
}
