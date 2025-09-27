#[derive(Debug, Clone, Copy)]
pub struct HCEConfig {
    pub tempo_bonus: i16,

    // Pawn structure
    pub doubled_pawn_penalty: i16,
    pub tripled_pawn_penalty: i16,
    pub isolated_pawn_penalty: i16,
    pub passed_pawn_linear: i16, // Linear component of passed pawn bonus
    pub passed_pawn_quadratic: i16, // Quadratic component (rank-1)^2

    // Piece bonuses
    pub bishop_pair_bonus: i16,
    pub rook_open_file_bonus: i16,
    pub rook_semi_open_file_bonus: i16,
    pub rook_seventh_rank_bonus: i16,

    // Mobility multipliers
    pub knight_mobility_multiplier: i16,
    pub bishop_mobility_multiplier: i16,
    pub rook_mobility_multiplier: i16,
    pub queen_mobility_multiplier: i16,

    // King safety - Pawn shield
    pub king_shield_r1_bonus: i16, // pawns on 2nd/7th ranks
    pub king_shield_r2_bonus: i16, // pawns on 3rd/6th ranks

    // King safety - File penalties
    pub king_open_file_penalty: i16,      // no pawn on either side
    pub king_semi_open_file_penalty: i16, // no our pawn, enemy pawn exists
    pub king_thin_cover_penalty: i16,     // only 1 of our pawns in window

    // King safety - Attack pressure
    pub king_pressure_knight: i16,
    pub king_pressure_bishop: i16,
    pub king_pressure_rook: i16,
    pub king_pressure_queen: i16,
    pub king_pressure_pawn: i16,

    // King safety - Positional
    pub king_central_penalty: i16, // penalty for central king in middlegame
    pub king_activity_bonus: i16,  // endgame king activity multiplier
}

impl Default for HCEConfig {
    fn default() -> Self {
        Self {
            tempo_bonus: 10,

            doubled_pawn_penalty: 30,
            tripled_pawn_penalty: 60,
            isolated_pawn_penalty: 39,
            passed_pawn_linear: 7,    // Linear growth per rank
            passed_pawn_quadratic: 3, // Quadratic acceleration

            bishop_pair_bonus: 50,
            rook_open_file_bonus: 15,
            rook_semi_open_file_bonus: 10,
            rook_seventh_rank_bonus: 20,

            // Mobility multipliers
            knight_mobility_multiplier: 5,
            bishop_mobility_multiplier: 3,
            rook_mobility_multiplier: 3,
            queen_mobility_multiplier: 1,

            // King safety
            king_shield_r1_bonus: 12,
            king_shield_r2_bonus: 6,
            king_open_file_penalty: 24,
            king_semi_open_file_penalty: 12,
            king_thin_cover_penalty: 6,
            king_pressure_knight: 12,
            king_pressure_bishop: 14,
            king_pressure_rook: 18,
            king_pressure_queen: 22,
            king_pressure_pawn: 8,
            king_central_penalty: 20,
            king_activity_bonus: 14,
        }
    }
}
