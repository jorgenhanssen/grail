pub const MAX_CONTINUATION_LOOKBACK: usize = 4;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineConfig {
    pub hash_size: i32,
    pub threads: usize,
    pub move_overhead: i32,
    pub multi_pv: u8,
    pub syzygy_path: String,
    pub syzygy_probe_depth: u8,

    pub aspiration_window_size: i16,
    pub aspiration_window_widen: i16,
    pub aspiration_window_depth: u8,
    pub aspiration_window_retries: i16,
    pub aspiration_score_divisor: i32,

    pub history_max_value: i32,
    pub history_prune_depth_multiplier: i16,
    pub history_bonus_multiplier: i32,
    pub history_malus_multiplier: i32,

    pub capture_history_max_value: i32,
    pub capture_history_bonus_multiplier: i32,
    pub capture_history_malus_multiplier: i32,

    pub continuation_max_value: i32,
    pub continuation_max_moves: usize,
    pub continuation_bonus_multiplier: i32,
    pub continuation_malus_multiplier: i32,

    // For move ordering
    // TODO: add an "ordering" prefix to these
    pub quiet_check_bonus: i16,
    pub quiet_check_see_margin: i16,
    pub bad_quiet_threshold: i16,
    pub escape_divisor: i16,
    pub unsafe_square_divisor: i16,

    pub lmr_divisor: i32,

    pub reduction_cut_node: u16,
    pub reduction_not_improving: u16,
    pub reduction_quiets_if_tt_capture: u16,
    pub anti_reduction_near_root: u16,
    pub anti_reduction_pv_node: u16,
    pub anti_reduction_pv_move: u16,
    pub anti_reduction_tactical: u16,
    pub anti_reduction_threat: u16,
    pub anti_reduction_check: u16,

    pub reduction_history_divisor: i32,
    pub reduction_capture_history_divisor: i32,
    pub reduction_cont_hist_divisor: i32,

    pub nmp_min_depth: u8,
    pub nmp_base_reduction: u8,
    pub nmp_depth_divisor: u8,
    pub nmp_eval_margin: i16,

    pub lmp_max_depth: u8,
    pub lmp_base_moves: i32,
    pub lmp_depth_multiplier: i32,
    pub lmp_improving_reduction: i32,

    pub futility_max_depth: u8,
    pub futility_base_margin: i16,
    pub futility_depth_multiplier: i16,

    pub rfp_max_depth: u8,
    pub rfp_base_margin: i16,
    pub rfp_depth_multiplier: i16,
    pub rfp_improving_bonus: i16,

    pub razor_max_depth: u8,
    pub razor_base_margin: i16,
    pub razor_depth_coefficient: i16,

    pub qs_delta_margin: i16,
    pub qs_delta_material_threshold: i16,

    pub iir_reduction: u8,
    pub iir_min_depth: u8,

    pub see_capture_max_depth: u8,
    pub see_capture_depth_margin: i16,
    pub see_capture_min_attacker_value: i16,
    pub see_quiet_max_depth: u8,
    pub see_quiet_depth_multiplier: i16,

    pub singular_min_depth: u8,
    pub singular_depth_margin: u8,
    pub singular_beta_margin: i16,
    pub double_ext_margin: i16,
    pub double_ext_overshoot_penalty: i16,

    pub correction_history_max_correction: i32,
    pub correction_table_size: usize,
    pub correction_pawn_weight: i32,
    pub correction_minor_weight: i32,
    pub correction_nonpawn_weight: i32,
    pub correction_continuation_weight: i32,
    pub correction_combined_divisor: i32,
    pub correction_minor_update_weight: i32,
    pub correction_nonpawn_update_weight: i32,
    pub correction_continuation_update_weight: i32,
    pub correction_continuation_max_moves: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            hash_size: 256,
            threads: 1,
            move_overhead: 10,
            multi_pv: 1,
            syzygy_path: String::new(),
            syzygy_probe_depth: 1,

            aspiration_window_size: 40,
            aspiration_window_widen: 2,
            aspiration_window_depth: 5,
            aspiration_window_retries: 3,
            aspiration_score_divisor: 16384,

            history_max_value: 512,
            history_prune_depth_multiplier: -80,
            history_bonus_multiplier: 13,
            history_malus_multiplier: 4,

            capture_history_max_value: 512,
            capture_history_bonus_multiplier: 9,
            capture_history_malus_multiplier: 2,

            continuation_max_value: 512,
            continuation_max_moves: 4,
            continuation_bonus_multiplier: 9,
            continuation_malus_multiplier: 10,

            quiet_check_bonus: 1000,
            quiet_check_see_margin: 75,
            bad_quiet_threshold: -150,
            escape_divisor: 10,
            unsafe_square_divisor: 20,

            lmr_divisor: 220,

            reduction_cut_node: 1024,
            reduction_not_improving: 1024,
            reduction_quiets_if_tt_capture: 1024,
            anti_reduction_near_root: 1024,
            anti_reduction_pv_node: 1024,
            anti_reduction_pv_move: 1024,
            anti_reduction_tactical: 1024,
            anti_reduction_threat: 1024,
            anti_reduction_check: 1500,

            reduction_history_divisor: 987,
            reduction_capture_history_divisor: 820,
            reduction_cont_hist_divisor: 5030,

            nmp_min_depth: 4,
            nmp_base_reduction: 2,
            nmp_depth_divisor: 3,
            nmp_eval_margin: 200,

            lmp_max_depth: 8,
            lmp_base_moves: 2,
            lmp_depth_multiplier: 2,
            lmp_improving_reduction: 85,

            futility_max_depth: 4,
            futility_base_margin: 150,
            futility_depth_multiplier: 100,

            rfp_max_depth: 5,
            rfp_base_margin: 150,
            rfp_depth_multiplier: 100,
            rfp_improving_bonus: 50,

            razor_max_depth: 3,
            razor_base_margin: 512,
            razor_depth_coefficient: 293,

            qs_delta_margin: 200,
            qs_delta_material_threshold: 1500,

            iir_reduction: 1,
            iir_min_depth: 4,

            see_capture_max_depth: 6,
            see_capture_depth_margin: 75,
            see_capture_min_attacker_value: 200,
            see_quiet_max_depth: 8,
            see_quiet_depth_multiplier: 64,

            singular_min_depth: 6,
            singular_depth_margin: 3,
            singular_beta_margin: 200,
            double_ext_margin: 50,
            double_ext_overshoot_penalty: 2,

            correction_history_max_correction: 1024,
            correction_table_size: 16384,
            correction_pawn_weight: 10460,
            correction_minor_weight: 8136,
            correction_nonpawn_weight: 11468,
            correction_continuation_weight: 4472,
            correction_combined_divisor: 134500,
            correction_minor_update_weight: 150,
            correction_nonpawn_update_weight: 180,
            correction_continuation_update_weight: 160,
            correction_continuation_max_moves: 2,
        }
    }
}
