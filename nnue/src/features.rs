use cozy_chess::{BitBoard, Board, Color, File, Piece, Square};
use utils::board_metrics::BoardMetrics;

use crate::bitset;

const NUM_KING_BUCKETS: usize = 12;
pub const NUM_ACCUMULATOR_BUCKETS: usize = 2 * NUM_KING_BUCKETS; // 2 because mirroring
const NUM_PIECE_FEATURES: usize = Square::NUM * Piece::NUM * Color::NUM;
const NUM_SUPPORT_FEATURES: usize = Square::NUM * Color::NUM;
const NUM_THREAT_FEATURES: usize = Square::NUM * Color::NUM;
const NUM_TACTICAL_FEATURES: usize = NUM_SUPPORT_FEATURES + NUM_THREAT_FEATURES;

pub const NUM_FEATURES: usize = NUM_KING_BUCKETS * NUM_PIECE_FEATURES + NUM_TACTICAL_FEATURES;

/// Piece placements, support and threats from one side's perspective.
#[derive(Clone, Copy, Default)]
pub struct Features {
    // The accumulator stores pieces per king buckets for faster inference.
    // therefore we only need to store the pieces for a single king bucket.
    pieces: bitset!(NUM_PIECE_FEATURES),
    tactical: bitset!(NUM_TACTICAL_FEATURES),
    king_bucket: usize,
    mirrored: bool, // king mirror
}

impl Features {
    pub fn from_board(board: &Board, metrics: &BoardMetrics, perspective: Color) -> Self {
        let mirror = king_is_on_the_right(board, perspective);

        let mut features = Self {
            king_bucket: king_bucket(board, perspective),
            mirrored: mirror,
            ..Default::default()
        };

        // Piece placements (per king bucket)
        for color in Color::ALL {
            let side_offset = if color == perspective { 0 } else { Piece::NUM };
            for piece in Piece::ALL {
                let piece_idx = side_offset + piece as usize;
                for sq in board.colored_pieces(color, piece) {
                    let sq_idx = transform_square(sq, perspective, mirror) as usize;
                    features
                        .pieces
                        .set(sq_idx * (Piece::NUM * Color::NUM) + piece_idx);
                }
            }
        }

        // Support
        let (us_support, them_support) = from_perspective(metrics.support, perspective, mirror);
        features.tactical.set_u64(0, us_support.0);
        features.tactical.set_u64(1, them_support.0);

        // Threats
        let (us_threats, them_threats) = from_perspective(metrics.threats, perspective, mirror);
        features.tactical.set_u64(2, us_threats.0);
        features.tactical.set_u64(3, them_threats.0);

        features
    }

    pub fn accumulator_index(&self) -> usize {
        2 * self.king_bucket + usize::from(self.mirrored)
    }

    pub fn for_each_diff(&self, current: &Self, mut f: impl FnMut(usize, bool)) {
        let piece_offset = current.king_bucket * NUM_PIECE_FEATURES;
        self.pieces.for_each_diff(&current.pieces, |idx| {
            f(piece_offset + idx, current.pieces.get(idx));
        });
        self.tactical.for_each_diff(&current.tactical, |idx| {
            f(PIECE_FEATURES_END + idx, current.tactical.get(idx));
        });
    }

    pub fn for_each_active(&self, mut f: impl FnMut(usize)) {
        let piece_offset = self.king_bucket * NUM_PIECE_FEATURES;
        self.pieces.for_each_active(|idx| f(piece_offset + idx));
        self.tactical
            .for_each_active(|idx| f(PIECE_FEATURES_END + idx));
    }
}

// Exported to the analysis tool
pub const PIECE_FEATURES_START: usize = 0;
pub const PIECE_FEATURES_END: usize = NUM_KING_BUCKETS * NUM_PIECE_FEATURES;
pub const US_SUPPORT_START: usize = PIECE_FEATURES_END;
pub const US_SUPPORT_END: usize = US_SUPPORT_START + Square::NUM;
pub const THEM_SUPPORT_START: usize = US_SUPPORT_END;
pub const THEM_SUPPORT_END: usize = THEM_SUPPORT_START + Square::NUM;
pub const US_THREATS_START: usize = THEM_SUPPORT_END;
pub const US_THREATS_END: usize = US_THREATS_START + Square::NUM;
pub const THEM_THREATS_START: usize = US_THREATS_END;
pub const THEM_THREATS_END: usize = THEM_THREATS_START + Square::NUM;

/// Encodes a board position into a dense f32 feature array from a perspective.
pub fn encode_board(
    board: &Board,
    metrics: &BoardMetrics,
    perspective: Color,
) -> [f32; NUM_FEATURES] {
    let mut features = [0f32; NUM_FEATURES];

    Features::from_board(board, metrics, perspective)
        .for_each_active(|index| features[index] = 1.0);

    features
}

/// Encodes features into a packed bitset.
pub fn encode_board_bitset(
    board: &Board,
    metrics: &BoardMetrics,
    perspective: Color,
) -> bitset!(NUM_FEATURES) {
    let mut bitset: bitset!(NUM_FEATURES) = Default::default();

    Features::from_board(board, metrics, perspective).for_each_active(|idx| bitset.set(idx));
    bitset
}

/// Active feature indices for one perspective.
pub type FeatureIndices = smallvec::SmallVec<[u32; 128]>;

/// Encodes a list of indices of active features for one perspective.
pub fn encode_board_indices(
    board: &Board,
    metrics: &BoardMetrics,
    perspective: Color,
) -> FeatureIndices {
    let features = Features::from_board(board, metrics, perspective);

    let mut indices = FeatureIndices::new();
    features.for_each_active(|index| indices.push(index as u32));

    indices
}

/// King buckets divides the board into regions, and the king's location
/// in the region determines which set of accumulator weights are used.
/// https://chessprogramming.org/NNUE#King_Input_Buckets
///
/// Buckets based on PlentyChess:
/// https://github.com/Yoshie2000/PlentyChess/blob/main/src/nnue.h
#[rustfmt::skip]
const KING_BUCKETS: [u8; Square::NUM] = [
    0,  1,  2,  3,  3,  2,  1,  0,
    4,  5,  6,  7,  7,  6,  5,  4,
    8,  8,  9,  9,  9,  9,  8,  8,
    10, 10, 10, 10, 10, 10, 10, 10,
    11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11,
];

/// Which king bucket this position falls into (from "our" perspective).
fn king_bucket(board: &Board, perspective: Color) -> usize {
    KING_BUCKETS[board.king(perspective).relative_to(perspective) as usize] as usize
}

/// Returns true if the board should be mirrored based on the king's file.
/// This way the king will always be on the left side of the board.
/// This reduces the network's input space and is done in most engines.
/// https://chessprogramming.org/NNUE#horizontal-mirroring
fn king_is_on_the_right(board: &Board, perspective: Color) -> bool {
    board.king(perspective).file() >= File::E
}

/// Square as seen from our side: ranks flipped for black, files when mirroring.
fn transform_square(sq: Square, perspective: Color, mirror: bool) -> Square {
    let sq = sq.relative_to(perspective);
    if mirror { sq.flip_file() } else { sq }
}

/// (us, them) bitboards relative to perspective.
fn from_perspective(
    [white, black]: [BitBoard; Color::NUM],
    perspective: Color,
    mirror: bool,
) -> (BitBoard, BitBoard) {
    let (us, them) = match perspective {
        Color::White => (white, black),
        Color::Black => (black.flip_ranks(), white.flip_ranks()),
    };

    if mirror {
        (us.flip_files(), them.flip_files())
    } else {
        (us, them)
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
        "q1q1k3/8/8/8/8/8/8/QQ2K3 w - - 0 1",                       // Promoted queens
        "4k3/8/8/3q1p2/2P1N3/2N5/8/4K3 w - - 0 1",                  // Support and threats
    ];

    #[test]
    fn test_encode_board_and_bitset_are_consistent() {
        for fen in TEST_POSITIONS {
            let board: Board = fen.parse().unwrap();
            let metrics = BoardMetrics::new(&board);

            for perspective in [Color::White, Color::Black] {
                let features = encode_board(&board, &metrics, perspective);

                let bitset = encode_board_bitset(&board, &metrics, perspective);

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

    #[test]
    fn test_white_king_mirroring() {
        // King on E1 should encode to D1 for white.
        let board: Board = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse()
            .unwrap();

        let metrics = BoardMetrics::new(&board);
        let features = encode_board(&board, &metrics, Color::White);

        let king = Piece::King as usize;
        let per_square = Piece::NUM * Color::NUM;
        // e1 (and mirrored d1) should both be in bucket 3.
        let offset = 3 * NUM_PIECE_FEATURES;
        assert_eq!(
            features[offset + Square::D1 as usize * per_square + king],
            1.0
        );
        assert_eq!(
            features[offset + Square::E1 as usize * per_square + king],
            0.0
        );
    }
}
