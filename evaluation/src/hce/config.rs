#[derive(Debug, Clone, Copy)]
pub struct HCEConfig {
    pub tempo_bonus: i16,

    // Pawn structure
    pub doubled_pawn_penalty: i16,
    pub tripled_pawn_penalty: i16,
    pub isolated_pawn_penalty: i16,
    pub backward_pawn_penalty: i16,
    pub backward_pawn_half_open_penalty: i16, // Extra penalty if on half-open file
    pub passed_pawn_linear: i16,              // Linear component of passed pawn bonus
    pub passed_pawn_quadratic: i16,           // Quadratic component (rank-1)^2
    pub pawn_storm_bonus: i16,                // Bonus per rank for pawns storming enemy king

    // Piece bonuses
    pub bishop_pair_bonus: i16,
    pub rook_open_file_bonus: i16,
    pub rook_semi_open_file_bonus: i16,
    pub rook_seventh_rank_bonus: i16,

    // Space advantage
    pub space_multiplier: i16,

    // Piece coordination
    pub supported_minor_bonus: i16, // bonus for defended knights/bishops
    pub supported_major_bonus: i16, // bonus for defended rooks/queens

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

    // Threats
    pub threats_multiplier: i16,
}
