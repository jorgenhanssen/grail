use cozy_chess::Board;
use rand::RngExt;
use utils::{Book, collect_legal_moves, has_legal_moves};

pub struct OpeningSource {
    pub book: Option<Book>,
    pub random_plies: usize,
}

impl OpeningSource {
    pub fn next_opening(&self) -> Board {
        let mut rng = rand::rng();

        'attempt: loop {
            let mut board = match &self.book {
                Some(book) => book.random_position(),
                None => Board::default(),
            };

            if self.random_plies == 0 {
                return board;
            }

            // Randomize parity so both colors get the tempo at the first move
            let plies = self.random_plies + rng.random_range(0..=1);

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
}
