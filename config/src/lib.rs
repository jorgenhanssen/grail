use uci::UciOptionType;

mod macros;
mod param;

pub use param::ConfigParam;

use macros::define_config;

pub const MAX_CONTINUATION_LOOKBACK: usize = 4;

define_config!(
    // Public UCI options
    (hash_size: i32, "Hash", UciOptionType::Spin { min: 1, max: 16384 }, 256, true),
    (threads: usize, "Threads", UciOptionType::Spin { min: 1, max: 256 }, 1, true),
    (move_overhead: i32, "Move Overhead", UciOptionType::Spin { min: 0, max: 5000 }, 10, true),
    (multi_pv: u8, "MultiPV", UciOptionType::Spin { min: 1, max: 64 }, 1, true),
    (syzygy_path: String, "SyzygyPath", UciOptionType::String, String::new(), true),
    (syzygy_probe_depth: u8, "SyzygyProbeDepth", UciOptionType::Spin { min: 1, max: 100 }, 1, true),

    // Tuning options
    (aspiration_window_size: i16, "Aspiration Window Size", UciOptionType::Spin { min: 10, max: 100 }, 40, cfg!(feature = "tuning")),
    (aspiration_window_widen: i16, "Aspiration Window Widening", UciOptionType::Spin { min: 2, max: 4 }, 2, cfg!(feature = "tuning")),
    (aspiration_window_depth: u8, "Aspiration Window Depth", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (aspiration_window_retries: i16, "Aspiration Window Retries", UciOptionType::Spin { min: 1, max: 5 }, 3, cfg!(feature = "tuning")),
    (aspiration_score_divisor: i32, "Aspiration Score Divisor", UciOptionType::Spin { min: 1024, max: 65536 }, 16384, cfg!(feature = "tuning")),

    (history_max_value: i32, "History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (history_prune_depth_multiplier: i16, "History Prune Depth Multiplier", UciOptionType::Spin { min: -256, max: 0 }, -80, cfg!(feature = "tuning")),
    (history_bonus_multiplier: i32, "History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 13, cfg!(feature = "tuning")),
    (history_malus_multiplier: i32, "History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 4, cfg!(feature = "tuning")),

    (capture_history_max_value: i32, "Capture History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (capture_history_bonus_multiplier: i32, "Capture History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 9, cfg!(feature = "tuning")),
    (capture_history_malus_multiplier: i32, "Capture History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 2, cfg!(feature = "tuning")),

    (continuation_max_value: i32, "Continuation Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (continuation_max_moves: usize, "Continuation Max Moves", UciOptionType::Spin { min: 1, max: MAX_CONTINUATION_LOOKBACK as i32 }, 4, cfg!(feature = "tuning")),
    (continuation_bonus_multiplier: i32, "Continuation Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 9, cfg!(feature = "tuning")),
    (continuation_malus_multiplier: i32, "Continuation Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")),

    (quiet_check_bonus: i16, "Quiet Check Bonus", UciOptionType::Spin { min: 0, max: 2000 }, 1000, cfg!(feature = "tuning")),
    (quiet_check_see_margin: i16, "Quiet Check SEE Margin", UciOptionType::Spin { min: 0, max: 300 }, 75, cfg!(feature = "tuning")),
    (bad_quiet_threshold: i16, "Bad Quiet Threshold", UciOptionType::Spin { min: -512, max: 0 }, -150, cfg!(feature = "tuning")),
    (escape_divisor: i16, "Escape Divisor", UciOptionType::Spin { min: 1, max: 100 }, 10, cfg!(feature = "tuning")),
    (unsafe_square_divisor: i16, "Unsafe Square Divisor", UciOptionType::Spin { min: 1, max: 100 }, 20, cfg!(feature = "tuning")),

    (lmr_divisor: i32, "LMR Divisor", UciOptionType::Spin { min: 100, max: 400 }, 220, cfg!(feature = "tuning")),

    (reduction_cut_node: u16, "Reduction Cut Node", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (reduction_not_improving: u16, "Reduction Not Improving", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (reduction_quiets_if_tt_capture: u16, "Reduction Quiets If TT Capture", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_near_root: u16, "Anti Reduction Near Root", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_pv_node: u16, "Anti Reduction PV Node", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_pv_move: u16, "Anti Reduction PV Move", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_tt_pv: u16, "Anti Reduction TT PV", UciOptionType::Spin { min: 0, max: 4096 }, 512, cfg!(feature = "tuning")),
    (anti_reduction_tactical: u16, "Anti Reduction Tactical", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_threat: u16, "Anti Reduction Threat", UciOptionType::Spin { min: 0, max: 4096 }, 1024, cfg!(feature = "tuning")),
    (anti_reduction_check: u16, "Anti Reduction Check", UciOptionType::Spin { min: 0, max: 4096 }, 1500, cfg!(feature = "tuning")),

    (reduction_history_divisor: i32, "Reduction History Divisor", UciOptionType::Spin { min: 64, max: 16384 }, 987, cfg!(feature = "tuning")),
    (reduction_capture_history_divisor: i32, "Reduction Capture History Divisor", UciOptionType::Spin { min: 64, max: 16384 }, 820, cfg!(feature = "tuning")),
    (reduction_cont_hist_divisor: i32, "Reduction Cont Hist Divisor", UciOptionType::Spin { min: 64, max: 16384 }, 5030, cfg!(feature = "tuning")),

    (nmp_min_depth: u8, "NMP Min Depth", UciOptionType::Spin { min: 2, max: 10 }, 4, cfg!(feature = "tuning")),
    (nmp_base_reduction: u8, "NMP Base Reduction", UciOptionType::Spin { min: 1, max: 10 }, 2, cfg!(feature = "tuning")),
    (nmp_depth_divisor: u8, "NMP Depth Divisor", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),
    (nmp_eval_margin: i16, "NMP Eval Margin", UciOptionType::Spin { min: 0, max: 500 }, 200, cfg!(feature = "tuning")),

    (lmp_max_depth: u8, "LMP Max Depth", UciOptionType::Spin { min: 0, max: 20 }, 8, cfg!(feature = "tuning")),
    (lmp_base_moves: i32, "LMP Base Moves", UciOptionType::Spin { min: 1, max: 10 }, 2, cfg!(feature = "tuning")),
    (lmp_depth_multiplier: i32, "LMP Depth Multiplier", UciOptionType::Spin { min: 1, max: 10 }, 2, cfg!(feature = "tuning")),
    (lmp_improving_reduction: i32, "LMP Improving Reduction", UciOptionType::Spin { min: 50, max: 100 }, 85, cfg!(feature = "tuning")),

    (futility_max_depth: u8, "Futility Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 4, cfg!(feature = "tuning")),
    (futility_base_margin: i16, "Futility Base Margin", UciOptionType::Spin { min: 10, max: 300 }, 150, cfg!(feature = "tuning")),
    (futility_depth_multiplier: i16, "Futility Depth Multiplier", UciOptionType::Spin { min: 10, max: 200 }, 100, cfg!(feature = "tuning")),

    (rfp_max_depth: u8, "RFP Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (rfp_base_margin: i16, "RFP Base Margin", UciOptionType::Spin { min: 10, max: 300 }, 150, cfg!(feature = "tuning")),
    (rfp_depth_multiplier: i16, "RFP Depth Multiplier", UciOptionType::Spin { min: 10, max: 200 }, 100, cfg!(feature = "tuning")),
    (rfp_improving_bonus: i16, "RFP Improving Bonus", UciOptionType::Spin { min: 10, max: 100 }, 50, cfg!(feature = "tuning")),

    (razor_max_depth: u8, "Razor Max Depth", UciOptionType::Spin { min: 1, max: 5 }, 3, cfg!(feature = "tuning")),
    (razor_base_margin: i16, "Razor Base Margin", UciOptionType::Spin { min: 100, max: 800 }, 512, cfg!(feature = "tuning")),
    (razor_depth_coefficient: i16, "Razor Depth Coefficient", UciOptionType::Spin { min: 100, max: 500 }, 293, cfg!(feature = "tuning")),

    (qs_delta_margin: i16, "QS Delta Margin", UciOptionType::Spin { min: 10, max: 500 }, 200, cfg!(feature = "tuning")),
    (qs_delta_material_threshold: i16, "QS Delta Material Threshold", UciOptionType::Spin { min: 100, max: 3000 }, 1500, cfg!(feature = "tuning")),

    (iir_reduction: u8, "IIR Reduction", UciOptionType::Spin { min: 0, max: 4 }, 1, cfg!(feature = "tuning")),
    (iir_min_depth: u8, "IIR Min Depth", UciOptionType::Spin { min: 2, max: 10 }, 4, cfg!(feature = "tuning")),

    (see_capture_max_depth: u8, "SEE Capture Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 6, cfg!(feature = "tuning")),
    (see_capture_depth_margin: i16, "SEE Capture Depth Margin", UciOptionType::Spin { min: 10, max: 150 }, 75, cfg!(feature = "tuning")),
    (see_capture_min_attacker_value: i16, "SEE Capture Min Attacker Value", UciOptionType::Spin { min: 0, max: 500 }, 200, cfg!(feature = "tuning")),

    (see_quiet_max_depth: u8, "SEE Quiet Max Depth", UciOptionType::Spin { min: 1, max: 12 }, 8, cfg!(feature = "tuning")),
    (see_quiet_depth_multiplier: i16, "SEE Quiet Depth Multiplier", UciOptionType::Spin { min: 10, max: 150 }, 64, cfg!(feature = "tuning")),

    (singular_min_depth: u8, "Singular Min Depth", UciOptionType::Spin { min: 4, max: 12 }, 6, cfg!(feature = "tuning")),
    (singular_depth_margin: u8, "Singular Depth Margin", UciOptionType::Spin { min: 1, max: 5 }, 3, cfg!(feature = "tuning")),
    (singular_beta_margin: i16, "Singular Beta Margin", UciOptionType::Spin { min: 100, max: 500 }, 200, cfg!(feature = "tuning")),
    (double_ext_margin: i16, "Double Extension Margin", UciOptionType::Spin { min: 10, max: 100 }, 50, cfg!(feature = "tuning")),
    (double_ext_overshoot_penalty: i16, "Double Extension Overshoot Penalty", UciOptionType::Spin { min: 0, max: 3 }, 2, cfg!(feature = "tuning")),

    (correction_history_max_correction: i32, "Correction History Max Correction", UciOptionType::Spin { min: 128, max: 2048 }, 1024, cfg!(feature = "tuning")),
    (correction_table_size: usize, "Correction Table Size", UciOptionType::Spin { min: 1024, max: 65536 }, 16384, cfg!(feature = "tuning")),

    (correction_pawn_weight: i32, "Correction Pawn Weight", UciOptionType::Spin { min: 0, max: 20000 }, 10460, cfg!(feature = "tuning")),
    (correction_minor_weight: i32, "Correction Minor Weight", UciOptionType::Spin { min: 0, max: 20000 }, 8136, cfg!(feature = "tuning")),
    (correction_nonpawn_weight: i32, "Correction NonPawn Weight", UciOptionType::Spin { min: 0, max: 20000 }, 11468, cfg!(feature = "tuning")),
    (correction_continuation_weight: i32, "Correction Continuation Weight", UciOptionType::Spin { min: 0, max: 20000 }, 4472, cfg!(feature = "tuning")),
    (correction_combined_divisor: i32, "Correction Combined Divisor", UciOptionType::Spin { min: 1024, max: 500000 }, 134500, cfg!(feature = "tuning")),
    (correction_minor_update_weight: i32, "Correction Minor Update Weight", UciOptionType::Spin { min: 64, max: 256 }, 150, cfg!(feature = "tuning")),
    (correction_nonpawn_update_weight: i32, "Correction NonPawn Update Weight", UciOptionType::Spin { min: 64, max: 256 }, 180, cfg!(feature = "tuning")),
    (correction_continuation_update_weight: i32, "Correction Continuation Update Weight", UciOptionType::Spin { min: 64, max: 256 }, 160, cfg!(feature = "tuning")),
    (correction_continuation_max_moves: usize, "Correction Continuation Max Moves", UciOptionType::Spin { min: 1, max: MAX_CONTINUATION_LOOKBACK as i32 }, 2, cfg!(feature = "tuning")),
);
