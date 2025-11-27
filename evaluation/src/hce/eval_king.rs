use super::HCEConfig;
use crate::hce::context::EvalContext;
use cozy_chess::{
    get_bishop_moves, get_knight_moves, get_rook_moves, BitBoard, Color, File, Piece, Rank, Square,
};

// Sum the king-safety bits: shield, files, ring pressure, center, activity.
// Middlegame terms matter more; king activity matters more in the endgame.
pub(super) fn evaluate(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let mut cp = 0i16;
    cp += pawn_shield_phase_bonus(ctx, color, config);
    cp += king_file_phase_penalty(ctx, color, config);
    cp += king_ring_phase_pressure(ctx, color, config);
    cp += central_king_phase_penalty(ctx, color, config);
    cp += endgame_king_activity(ctx, color, config);
    cp
}

// Pawns in front act as a shield. Count our pawns in the 3-file window on ranks 2/3 (7/6),
// weight the closer ones more. Most relevant in the opening/middlegame.
fn pawn_shield_phase_bonus(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let pawns = board.pieces(Piece::Pawn);
    let my_pawns = pawns & board.colors(color);
    let king_sq = board.king(color);
    let files_window = king_files_window(king_sq.file());
    let (front_rank_1, front_rank_2) = if color == Color::White {
        (Rank::Second, Rank::Third)
    } else {
        (Rank::Seventh, Rank::Sixth)
    };
    let shield_r1 = (my_pawns & files_window & front_rank_1.bitboard()).len() as i16;
    let shield_r2 = (my_pawns & files_window & front_rank_2.bitboard()).len() as i16;
    let shield_score =
        shield_r1 * config.king_shield_r1_bonus + shield_r2 * config.king_shield_r2_bonus;
    ((shield_score as f32) * ctx.phase).round() as i16
}

// Open/semi-open files next to the king increase exposure.
// Penalize no own pawns (fully open worse) and thin cover. Mostly an opening/middlegame concern.
fn king_file_phase_penalty(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let board = ctx.position.board;
    let king_sq = board.king(color);
    let files_window = king_files_window(king_sq.file());
    let pawns = board.pieces(Piece::Pawn);
    let my_pawns = pawns & board.colors(color);
    let their_pawns = pawns & board.colors(!color);
    let our_file_pawns = (my_pawns & files_window).len();
    let their_file_pawns = (their_pawns & files_window).len();
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
    -((file_penalty as f32) * ctx.phase).round() as i16
}

// Attack density around the king correlates with danger.
// Count enemy attacks into a 2-square ring; weight by piece and pawn diagonals. Stronger in the middlegame.
fn king_ring_phase_pressure(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    let enemy = !color;
    let board = ctx.position.board;
    let king_sq = board.king(color);
    let king_zone = KING_ZONES[king_sq as usize];
    let mut pressure = 0i16;

    let all_pieces = board.occupied();
    let enemy_pieces = board.colors(enemy);

    // Knights
    let knights = board.pieces(Piece::Knight) & enemy_pieces;
    for sq in knights {
        let attacks = get_knight_moves(sq) & king_zone;
        if !attacks.is_empty() {
            pressure += config.king_pressure_knight * (attacks.len() as i16);
        }
    }
    // Bishops
    let bishops = board.pieces(Piece::Bishop) & enemy_pieces;
    for sq in bishops {
        let attacks = get_bishop_moves(sq, all_pieces) & king_zone;
        if !attacks.is_empty() {
            pressure += config.king_pressure_bishop * (attacks.len() as i16);
        }
    }
    // Rooks
    let rooks = board.pieces(Piece::Rook) & enemy_pieces;
    for sq in rooks {
        let attacks = get_rook_moves(sq, all_pieces) & king_zone;
        if !attacks.is_empty() {
            pressure += config.king_pressure_rook * (attacks.len() as i16);
        }
    }
    // Queens
    let queens = board.pieces(Piece::Queen) & enemy_pieces;
    for sq in queens {
        let attacks =
            (get_bishop_moves(sq, all_pieces) | get_rook_moves(sq, all_pieces)) & king_zone;
        if !attacks.is_empty() {
            pressure += config.king_pressure_queen * (attacks.len() as i16);
        }
    }
    // Pawns
    let pawns = board.pieces(Piece::Pawn) & enemy_pieces;
    for sq in pawns {
        let f = sq.file() as i8;
        let r = sq.rank() as i8;
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
                let idx = (nr as usize) * 8 + (nf as usize);
                let attack_sq = Square::index(idx);
                if king_zone.has(attack_sq) {
                    count_in_zone += 1;
                }
            }
        }
        if count_in_zone > 0 {
            pressure += config.king_pressure_pawn * count_in_zone;
        }
    }

    -((pressure as f32) * ctx.phase).round() as i16
}

// Central back-rank kings are unsafe in the middlegame.
// If the king sits on c–f files on the home two ranks during the middlegame, ramp a penalty.
fn central_king_phase_penalty(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    if ctx.phase <= 0.5 {
        return 0;
    }

    let sq = ctx.position.board.king(color);

    let file_idx = sq.file() as i32;
    let rank_idx = sq.rank() as i32;
    let is_central_file = (2..=5).contains(&file_idx); // c,d,e,f
    let is_back_two = if color == Color::White {
        rank_idx <= 1
    } else {
        rank_idx >= 6
    };
    if is_central_file && is_back_two {
        -((config.king_central_penalty as f32 * ctx.phase) as i16)
    } else {
        0
    }
}

// Active kings decide endgames. Reward closeness to d4/e4/d5/e5 by Manhattan distance.
// Matters more as the game simplifies toward the endgame.
fn endgame_king_activity(ctx: &EvalContext, color: Color, config: &HCEConfig) -> i16 {
    if ctx.phase >= 0.4 {
        return 0;
    }

    let king_sq = ctx.position.board.king(color);

    let file = king_sq.file() as i32;
    let rank = king_sq.rank() as i32;
    let d = ((file - 3).abs() + (rank - 3).abs())
        .min((file - 4).abs() + (rank - 3).abs())
        .min((file - 3).abs() + (rank - 4).abs())
        .min((file - 4).abs() + (rank - 4).abs()) as i16;
    ((config.king_activity_bonus - d) as f32 * 2.0 * (1.0 - ctx.phase)).round() as i16
}

// 3-file mask around the king: the king's file plus its neighbors.
fn king_files_window(file: File) -> BitBoard {
    file.bitboard() | file.adjacent()
}

const KING_ZONE_RADIUS: i8 = 2;
const KING_ZONES: [BitBoard; Square::NUM] = {
    let mut zones = [BitBoard::EMPTY; Square::NUM];
    let mut i = 0;
    while i < Square::NUM {
        let king_file = (i % 8) as i8;
        let king_rank = (i / 8) as i8;

        let mut zone_bits = 0u64;
        let mut rank_offset = -KING_ZONE_RADIUS;
        while rank_offset <= KING_ZONE_RADIUS {
            let mut file_offset = -KING_ZONE_RADIUS;
            while file_offset <= KING_ZONE_RADIUS {
                let new_file = king_file + file_offset;
                let new_rank = king_rank + rank_offset;

                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    zone_bits |= 1u64 << (new_rank * 8 + new_file) as u64;
                }
                file_offset += 1;
            }
            rank_offset += 1;
        }
        zones[i] = BitBoard(zone_bits);
        i += 1;
    }
    zones
};
