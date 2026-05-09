/// Generate a Zobrist table at compile time using xorshift64.
///
/// <https://en.wikipedia.org/wiki/Xorshift>
/// <https://www.chessprogramming.org/Zobrist_Hashing>
pub const fn generate_zobrist_table<const N: usize>(seed: u64) -> [u64; N] {
    let mut s = seed;
    let mut table = [0u64; N];
    let mut i = 0;
    while i < N {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        table[i] = s;
        i += 1;
    }
    table
}

/// Compute a Zobrist key over a board (or specific color's pieces)
#[macro_export]
macro_rules! zobrist_key {
    ($board:expr, $table:expr, $pieces:expr) => {{
        use cozy_chess::{Color, Square};

        let pieces = $pieces;
        let mut key = 0u64;
        for color in Color::ALL {
            for (i, &piece) in pieces.iter().enumerate() {
                for sq in $board.colored_pieces(color, piece) {
                    let idx =
                        color as usize * pieces.len() * Square::NUM + i * Square::NUM + sq as usize;
                    key ^= $table[idx];
                }
            }
        }
        key
    }};
    ($board:expr, $table:expr, $pieces:expr, $color:expr) => {{
        use cozy_chess::Square;

        let pieces = $pieces;
        let mut key = 0u64;
        for (i, &piece) in pieces.iter().enumerate() {
            for sq in $board.colored_pieces($color, piece) {
                let idx = i * Square::NUM + sq as usize;
                key ^= $table[idx];
            }
        }
        key
    }};
}
