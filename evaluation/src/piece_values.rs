use chess::{Board, Piece, ALL_PIECES};

#[derive(Debug, Clone, Copy)]
pub struct PieceValues {
    pub pawn_value_mg: f32,
    pub pawn_value_eg: f32,
    pub knight_value_mg: f32,
    pub knight_value_eg: f32,
    pub bishop_value_mg: f32,
    pub bishop_value_eg: f32,
    pub rook_value_mg: f32,
    pub rook_value_eg: f32,
    pub queen_value_mg: f32,
    pub queen_value_eg: f32,
}

impl PieceValues {
    pub fn get(&self, piece: Piece, phase: f32) -> i16 {
        let (mg, eg) = match piece {
            Piece::Pawn => (self.pawn_value_mg, self.pawn_value_eg),
            Piece::Knight => (self.knight_value_mg, self.knight_value_eg),
            Piece::Bishop => (self.bishop_value_mg, self.bishop_value_eg),
            Piece::Rook => (self.rook_value_mg, self.rook_value_eg),
            Piece::Queen => (self.queen_value_mg, self.queen_value_eg),
            Piece::King => return 0, // Cut early for king
        };
        ((mg * phase) + (eg * (1.0 - phase))).round() as i16
    }

    pub fn total_material(&self, board: &Board, phase: f32) -> i16 {
        let mut material = 0;
        for piece in ALL_PIECES {
            material += self.get(piece, phase) * (board.pieces(piece).popcnt() as i16);
        }
        material
    }
}
