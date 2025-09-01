use chess::{BitBoard, Board, Color, Piece, EMPTY};

use crate::piece_values::piece_value;

#[inline(always)]
pub(super) fn evaluate(board: &Board, color: Color, phase: f32) -> i16 {
    let king_square = board.king_square(color);
    let mut cp = 0i16;

    // King safety (opening/middlegame)
    let king_zone = KING_ZONES[king_square.to_index()];
    let enemy_color = !color;
    let enemy_pieces = board.color_combined(enemy_color);

    let queens = (enemy_pieces & board.pieces(Piece::Queen) & king_zone).popcnt() as i16;
    let rooks = (enemy_pieces & board.pieces(Piece::Rook) & king_zone).popcnt() as i16;
    let bishops = (enemy_pieces & board.pieces(Piece::Bishop) & king_zone).popcnt() as i16;
    let knights = (enemy_pieces & board.pieces(Piece::Knight) & king_zone).popcnt() as i16;
    let pawns = (enemy_pieces & board.pieces(Piece::Pawn) & king_zone).popcnt() as i16;

    let mut safety_penalty = 0i16;
    safety_penalty -= queens * piece_value(Piece::Queen, phase);
    safety_penalty -= rooks * piece_value(Piece::Rook, phase);
    safety_penalty -= bishops * piece_value(Piece::Bishop, phase);
    safety_penalty -= knights * piece_value(Piece::Knight, phase);
    safety_penalty -= pawns * piece_value(Piece::Pawn, phase);

    // Let's do 30% of the value of the pieces
    cp += ((0.3 * (safety_penalty as f32)) * phase).round() as i16;

    // King activity (endgame)
    if phase < 0.4 {
        let file = board.king_square(color).get_file() as i32;
        let rank = board.king_square(color).get_rank() as i32;

        // Manhattan distance to d4(3,3), e4(4,3), d5(3,4), e5(4,4)
        let d = ((file - 3).abs() + (rank - 3).abs())
            .min((file - 4).abs() + (rank - 3).abs())
            .min((file - 3).abs() + (rank - 4).abs())
            .min((file - 4).abs() + (rank - 4).abs()) as i16;

        cp += ((14 - d) as f32 * 2.0 * (1.0 - phase)).round() as i16;
    }

    cp
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
