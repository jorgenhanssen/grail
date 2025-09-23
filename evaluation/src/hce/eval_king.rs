use super::HCEConfig;
use chess::{get_adjacent_files, get_file, get_rank, BitBoard, Board, Color, Piece, Rank, EMPTY};

// Sum the king-safety bits: shield, files, ring pressure, center, activity.
// Middlegame terms matter more; king activity matters more in the endgame.
#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let mut cp = 0i16;
    cp += pawn_shield_phase_bonus(board, color, phase, config);
    cp += king_file_phase_penalty(board, color, phase, config);
    cp += king_ring_phase_pressure(board, color, phase, config);
    cp += central_king_phase_penalty(board, color, phase, config);
    cp += endgame_king_activity(board, color, phase, config);
    cp
}

// Pawns in front act as a shield. Count our pawns in the 3-file window on ranks 2/3 (7/6),
// weight the closer ones more. Most relevant in the opening/middlegame.
#[inline(always)]
fn pawn_shield_phase_bonus(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let my_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    let files_window = king_files_window(board.king_square(color));
    let (front_rank_1, front_rank_2) = if color == Color::White {
        (Rank::Second, Rank::Third)
    } else {
        (Rank::Seventh, Rank::Sixth)
    };
    let shield_r1 = (my_pawns & files_window & get_rank(front_rank_1)).popcnt() as i16;
    let shield_r2 = (my_pawns & files_window & get_rank(front_rank_2)).popcnt() as i16;
    let shield_score =
        shield_r1 * config.king_shield_r1_bonus + shield_r2 * config.king_shield_r2_bonus;
    ((shield_score as f32) * phase).round() as i16
}

// Open/semi-open files next to the king increase exposure.
// Penalize no own pawns (fully open worse) and thin cover. Mostly an opening/middlegame concern.
#[inline(always)]
fn king_file_phase_penalty(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let files_window = king_files_window(board.king_square(color));
    let my_pawns = board.pieces(Piece::Pawn) & board.color_combined(color);
    let their_pawns = board.pieces(Piece::Pawn) & board.color_combined(!color);
    let our_file_pawns = (my_pawns & files_window).popcnt();
    let their_file_pawns = (their_pawns & files_window).popcnt();
    let mut file_penalty = 0i16;
    if our_file_pawns == 0 {
        if their_file_pawns == 0 {
            file_penalty += config.king_open_file_penalty;
        } else {
            file_penalty += config.king_semi_open_file_penalty;
        }
    } else if our_file_pawns == 1 {
        file_penalty += config.king_thin_cover_penalty;
    }
    -((file_penalty as f32) * phase).round() as i16
}

// Attack density around the king correlates with danger.
// Count enemy attacks into a 2-square ring; weight by piece and pawn diagonals. Stronger in the middlegame.
#[inline(always)]
fn king_ring_phase_pressure(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    let enemy = !color;
    let occupied = *board.combined();
    let king_zone = KING_ZONES[board.king_square(color).to_index()];
    let mut pressure = 0i16;

    // Knights
    let knights = board.pieces(Piece::Knight) & board.color_combined(enemy);
    for sq in knights {
        let attacks = chess::get_knight_moves(sq) & king_zone;
        if attacks != EMPTY {
            pressure += config.king_pressure_knight * (attacks.popcnt() as i16);
        }
    }
    // Bishops
    let bishops = board.pieces(Piece::Bishop) & board.color_combined(enemy);
    for sq in bishops {
        let attacks = chess::get_bishop_moves(sq, occupied) & king_zone;
        if attacks != EMPTY {
            pressure += config.king_pressure_bishop * (attacks.popcnt() as i16);
        }
    }
    // Rooks
    let rooks = board.pieces(Piece::Rook) & board.color_combined(enemy);
    for sq in rooks {
        let attacks = chess::get_rook_moves(sq, occupied) & king_zone;
        if attacks != EMPTY {
            pressure += config.king_pressure_rook * (attacks.popcnt() as i16);
        }
    }
    // Queens
    let queens = board.pieces(Piece::Queen) & board.color_combined(enemy);
    for sq in queens {
        let attacks = (chess::get_bishop_moves(sq, occupied) | chess::get_rook_moves(sq, occupied))
            & king_zone;
        if attacks != EMPTY {
            pressure += config.king_pressure_queen * (attacks.popcnt() as i16);
        }
    }
    // Pawns
    let pawns = board.pieces(Piece::Pawn) & board.color_combined(enemy);
    for sq in pawns {
        let f = sq.get_file() as i8;
        let r = sq.get_rank() as i8;
        let deltas: &[(i8, i8)] = if enemy == Color::White {
            &[(1, 1), (-1, 1)]
        } else {
            &[(-1, -1), (1, -1)]
        };
        let mut count_in_zone = 0i16;
        for (df, dr) in deltas {
            let nf = f + df;
            let nr = r + dr;
            if (0i8..=7i8).contains(&nf) && (0i8..=7i8).contains(&nr) {
                let idx = (nr as u64) * 8 + (nf as u64);
                let bb = BitBoard(1u64 << idx);
                if (bb & king_zone) != EMPTY {
                    count_in_zone += 1;
                }
            }
        }
        if count_in_zone > 0 {
            pressure += config.king_pressure_pawn * count_in_zone;
        }
    }

    -((pressure as f32) * phase).round() as i16
}

// Central back-rank kings are unsafe in the middlegame.
// If the king sits on c–f files on the home two ranks during the middlegame, ramp a penalty.
#[inline(always)]
fn central_king_phase_penalty(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    if phase <= 0.5 {
        return 0;
    }
    let sq = board.king_square(color);
    let file_idx = sq.get_file() as i32;
    let rank_idx = sq.get_rank() as i32;
    let is_central_file = (2..=5).contains(&file_idx); // c,d,e,f
    let is_back_two = if color == Color::White {
        rank_idx <= 1
    } else {
        rank_idx >= 6
    };
    if is_central_file && is_back_two {
        -((config.king_central_penalty as f32 * phase) as i16)
    } else {
        0
    }
}

// Active kings decide endgames. Reward closeness to d4/e4/d5/e5 by Manhattan distance.
// Matters more as the game simplifies toward the endgame.
#[inline(always)]
fn endgame_king_activity(board: &Board, color: Color, phase: f32, config: &HCEConfig) -> i16 {
    if phase >= 0.4 {
        return 0;
    }
    let file = board.king_square(color).get_file() as i32;
    let rank = board.king_square(color).get_rank() as i32;
    let d = ((file - 3).abs() + (rank - 3).abs())
        .min((file - 4).abs() + (rank - 3).abs())
        .min((file - 3).abs() + (rank - 4).abs())
        .min((file - 4).abs() + (rank - 4).abs()) as i16;
    ((config.king_activity_bonus - d) as f32 * 2.0 * (1.0 - phase)).round() as i16
}

// 3-file mask around the king: the king's file plus its neighbors.
#[inline(always)]
fn king_files_window(sq: chess::Square) -> BitBoard {
    let f = sq.get_file();
    get_file(f) | get_adjacent_files(f)
}

const KING_ZONE_RADIUS: i8 = 2;
const KING_ZONES: [BitBoard; 64] = {
    let mut zones = [EMPTY; 64];
    let mut i = 0;
    while i < 64 {
        let king_file = (i % 8) as i8;
        let king_rank = (i / 8) as i8;

        let mut zone = EMPTY;
        let mut rank_offset = -KING_ZONE_RADIUS;
        while rank_offset <= KING_ZONE_RADIUS {
            let mut file_offset = -KING_ZONE_RADIUS;
            while file_offset <= KING_ZONE_RADIUS {
                let new_file = king_file + file_offset;
                let new_rank = king_rank + rank_offset;

                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    zone = BitBoard(zone.0 | (1u64 << (new_rank * 8 + new_file) as u64));
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        zones[i] = zone;
        i += 1;
    }
    zones
};
