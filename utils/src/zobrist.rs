use cozy_chess::{Board, Color, Piece, Square};

/// Precomputed Zobrist keys for pawn positions.
/// Used in correction history for indexing.
///
/// <https://www.chessprogramming.org/Zobrist_Hashing>
const PAWN_ZOBRIST: [u64; Square::NUM * Color::NUM] = {
    // From Norway with love :)
    let mut seed: u64 = 0xE2E4_D7D5_E4D5_D8D5;

    let mut table = [0u64; Square::NUM * Color::NUM];

    // Populate table with deterministic pseudo-random values
    // https://en.wikipedia.org/wiki/Xorshift
    let mut i = 0;
    while i < table.len() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        table[i] = seed;
        i += 1;
    }
    table
};

/// Compute a Zobrist hash key for the pawn structure.
/// Standard Zobrist hashing applied to pawn positions only.
///
/// <https://www.chessprogramming.org/Zobrist_Hashing>
pub fn pawn_key(board: &Board) -> u64 {
    let mut key = 0u64;

    for color in Color::ALL {
        let pawns = board.colored_pieces(color, Piece::Pawn);
        for sq in pawns {
            let idx = color as usize * Square::NUM + sq as usize;
            key ^= PAWN_ZOBRIST[idx];
        }
    }

    key
}
