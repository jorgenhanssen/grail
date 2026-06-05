use cozy_chess::Board;
use rand::RngExt;
use utils::{Book, collect_legal_moves, has_legal_moves};

pub enum OpeningSource {
    /// Openings from a book.
    Book(Book),
    /// Openings reached from startpos + `plies` (or `plies + 1`) random moves.
    Random { plies: usize },
}

impl OpeningSource {
    pub fn next_opening(&self) -> Board {
        match self {
            Self::Book(book) => book.random_position(),
            Self::Random { plies } => play_random_from_startpos(*plies),
        }
    }
}

fn play_random_from_startpos(plies: usize) -> Board {
    let mut rng = rand::rng();

    'attempt: loop {
        let mut board = Board::default();

        // Randomize parity so both colors get the tempo at the first
        let plies = plies + rng.random_range(0..=1);

        for _ in 0..plies {
            let moves = collect_legal_moves(&board);
            if moves.is_empty() {
                // The random walk reached checkmate/stalemate, try again
                continue 'attempt;
            }
            board.play_unchecked(moves[rng.random_range(0..moves.len())]);
        }

        if has_legal_moves(&board) {
            return board;
        }
    }
}
