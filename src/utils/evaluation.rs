use chess::{Board, BoardStatus, Color, MoveGen, Piece, Square};

// Return final evaluation (positive = good for White, negative = good for Black)
// Does not consider checkmate!
pub fn evaluate_board(board: &Board) -> f32 {
    match board.status() {
        BoardStatus::Checkmate => {
            // If it’s White to move and board is checkmated => White lost
            if board.side_to_move() == chess::Color::White {
                return -10_000.0;
            } else {
                return 10_000.0;
            }
        }
        BoardStatus::Stalemate => return 0.0,
        BoardStatus::Ongoing => {}
    }

    let mut score = evaluate_material(board);

    score += count_mobility(board, Color::White) as f32;
    score -= count_mobility(board, Color::Black) as f32;

    score += evaluate_pawn_structure(board, Color::White);
    score -= evaluate_pawn_structure(board, Color::Black);

    score
}

fn evaluate_material(board: &Board) -> f32 {
    let mut score = 0.0;
    let mut white_bishops = 0;
    let mut black_bishops = 0;

    for sq in chess::ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            let material = base_piece_value(piece);
            let pst = psq_bonus(piece, color, sq);

            // Count bishops while we're iterating
            if piece == Piece::Bishop {
                if color == Color::White {
                    white_bishops += 1;
                } else {
                    black_bishops += 1;
                }
            }

            // From White's perspective: White pieces add, Black pieces subtract.
            if color == Color::White {
                score += material + pst;
            } else {
                score -= material + pst;
            }
        }
    }

    // Add bishop pair bonus (typically around 0.5 pawns = 50 centipawns)
    const BISHOP_PAIR_BONUS: f32 = 50.0;
    if white_bishops >= 2 {
        score += BISHOP_PAIR_BONUS;
    }
    if black_bishops >= 2 {
        score -= BISHOP_PAIR_BONUS;
    }

    score
}

const WHITE_PAWN_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 1.0, 1.0, 2.0,
    3.0, 3.0, 2.0, 1.0, 1.0, 0.5, 0.5, 1.0, 2.5, 2.5, 1.0, 0.5, 0.5, 0.0, 0.0, 0.0, 2.0, 2.0, 0.0,
    0.0, 0.0, 0.5, -0.5, -1.0, 0.0, 0.0, -1.0, -0.5, 0.5, 0.5, 1.0, 1.0, -2.0, -2.0, 1.0, 1.0, 0.5,
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

const BLACK_PAWN_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_PAWN_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_KNIGHT_PST: [f32; 64] = [
    -5.0, -4.0, -2.0, -2.0, -2.0, -2.0, -4.0, -5.0, -4.0, -2.0, 0.0, 0.5, 0.5, 0.0, -2.0, -4.0,
    -2.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -2.0, -2.0, 0.0, 1.0, 2.0, 2.0, 1.0, 0.0, -2.0, -2.0, 0.0,
    1.0, 2.0, 2.0, 1.0, 0.0, -2.0, -2.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, -2.0, -4.0, -2.0, 0.0, 0.5,
    0.5, 0.0, -2.0, -4.0, -5.0, -4.0, -2.0, -2.0, -2.0, -2.0, -4.0, -5.0,
];

const BLACK_KNIGHT_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_KNIGHT_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_BISHOP_PST: [f32; 64] = [
    -2.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -2.0, -1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.5, -1.0, -1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.5, 0.5,
    1.0, 1.0, 0.5, 0.5, -1.0, -1.0, 0.0, 0.5, 1.0, 1.0, 0.5, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -1.0, -2.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -2.0,
];

const BLACK_BISHOP_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_BISHOP_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_ROOK_PST: [f32; 64] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, -0.5, 0.0, 0.0,
    0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0,
];

const BLACK_ROOK_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_ROOK_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_QUEEN_PST: [f32; 64] = [
    -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0,
    0.5, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, -0.5, 0.0, 0.5, 0.5, 0.5, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5,
    0.5, 0.5, 0.5, 0.0, -0.5, -1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0,
    0.0, 0.0, -1.0, -2.0, -1.0, -1.0, -0.5, -0.5, -1.0, -1.0, -2.0,
];

const BLACK_QUEEN_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_QUEEN_PST[63 - i];
        i += 1;
    }
    table
};

const WHITE_KING_PST: [f32; 64] = [
    -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0, -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0,
    -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0, -3.0, -4.0, -4.0, -5.0, -5.0, -4.0, -4.0, -3.0,
    -2.0, -3.0, -3.0, -4.0, -4.0, -3.0, -3.0, -2.0, -1.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0, -1.0,
    2.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 3.0, 1.0, 0.0, 0.0, 1.0, 3.0, 2.0,
];

const BLACK_KING_PST: [f32; 64] = {
    let mut table = [0.0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = -WHITE_KING_PST[63 - i];
        i += 1;
    }
    table
};

fn base_piece_value(piece: Piece) -> f32 {
    match piece {
        Piece::Pawn => 100.0,
        Piece::Knight => 320.0,
        Piece::Bishop => 330.0,
        Piece::Rook => 500.0,
        Piece::Queen => 900.0,
        Piece::King => 0.0, // King is treated specially
    }
}

fn psq_bonus(piece: Piece, color: Color, sq: Square) -> f32 {
    let idx = sq.to_index();
    match piece {
        Piece::Pawn => {
            if color == Color::White {
                WHITE_PAWN_PST[idx]
            } else {
                BLACK_PAWN_PST[idx]
            }
        }
        Piece::Knight => {
            if color == Color::White {
                WHITE_KNIGHT_PST[idx]
            } else {
                BLACK_KNIGHT_PST[idx]
            }
        }
        Piece::Bishop => {
            if color == Color::White {
                WHITE_BISHOP_PST[idx]
            } else {
                BLACK_BISHOP_PST[idx]
            }
        }
        Piece::Rook => {
            if color == Color::White {
                WHITE_ROOK_PST[idx]
            } else {
                BLACK_ROOK_PST[idx]
            }
        }
        Piece::Queen => {
            if color == Color::White {
                WHITE_QUEEN_PST[idx]
            } else {
                BLACK_QUEEN_PST[idx]
            }
        }
        Piece::King => {
            if color == Color::White {
                WHITE_KING_PST[idx]
            } else {
                BLACK_KING_PST[idx]
            }
        }
    }
}

fn count_mobility(board: &Board, color: Color) -> usize {
    if board.side_to_move() == color {
        // It's already our turn
        MoveGen::new_legal(board).count()
    } else {
        // Force a null move if possible
        if let Some(board_after_null) = board.null_move() {
            MoveGen::new_legal(&board_after_null).count()
        } else {
            // If null move is not available (e.g. in check),
            // fallback to 0 for now.
            0
        }
    }
}

fn evaluate_pawn_structure(board: &Board, color: Color) -> f32 {
    let mut score = 0.0;
    let pawns = board.pieces(Piece::Pawn) & board.color_combined(color);

    // File occupancy
    let mut files = [0; 8];
    for sq in pawns {
        let file = sq.get_file() as usize;
        files[file] += 1;
    }

    // check isolated pawns and doubled+ pawns
    for file in 0..8 {
        match files[file] {
            0 => continue,
            1 => {
                // Check for isolated pawns in the same loop
                let isolated = if file == 0 {
                    files[file + 1] == 0
                } else if file == 7 {
                    files[file - 1] == 0
                } else {
                    files[file - 1] == 0 && files[file + 1] == 0
                };
                if isolated {
                    score -= 30.0;
                }
            }
            2 => score -= 20.0, // doubled pawns
            _ => score -= 40.0, // tripled or more
        }
    }

    score
}
