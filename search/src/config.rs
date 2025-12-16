use std::str::FromStr;

use evaluation::PieceValues;
use hce::HCEConfig;
use uci::{UciOption, UciOptionType, UciOutput};

/// Helper to conditionally create UCI option metadata.
/// If `include` is false (e.g., tuning feature disabled), option won't appear in UCI.
fn uci(include: bool, name: &'static str, option_type: UciOptionType) -> Option<UciOption> {
    if include {
        Some(UciOption { name, option_type })
    } else {
        None
    }
}

/// Generates EngineConfig struct and UCI plumbing from a list of parameters.
///
/// Each entry: (field_name: Type, "UCI Name", UciOptionType, default_value, include_in_uci)
///
/// The macro generates:
/// - `EngineConfig` struct with all fields as `ConfigParam<T>`
/// - `Default` impl with specified defaults
/// - `update_from_uci()` to set values from UCI setoption commands
/// - `to_uci()` to send all options to the GUI
///
/// The `include` flag (often `cfg!(feature = "tuning")`) controls whether
/// the option is exposed via UCI. useful for hiding tuning params in release builds.
macro_rules! define_config {
    ($(($field:ident: $type:ty, $uci_name:expr, $uci_type:expr, $default:expr, $include:expr)),* $(,)?) => {
        #[derive(Debug, Clone)]
        pub struct EngineConfig {
            $(pub $field: ConfigParam<$type>,)*
        }

        impl Default for EngineConfig {
            fn default() -> Self {
                Self {
                    $($field: ConfigParam {
                        value: $default,
                        uci: uci($include, $uci_name, $uci_type),
                    },)*
                }
            }
        }

        impl EngineConfig {
            pub fn update_from_uci(&mut self, uci_name: &str, value: &str) -> Result<(), String> {
                // TODO: Empty name is a workaround for malformed setoption commands.
                // Consider adding InvalidCommand variant to UciInput instead.
                if uci_name.is_empty() {
                    return Err("Invalid setoption command: missing option name".to_string());
                }

                match uci_name {
                    $($uci_name if $include => self.$field.update_from_uci(value),)*
                    _ => Err(format!("Unknown option: {}", uci_name)),
                }
            }

            pub fn to_uci(&self, output: &std::sync::mpsc::Sender<UciOutput>) -> Result<(), std::sync::mpsc::SendError<UciOutput>> {
                $(
                    if self.$field.uci.is_some() {
                        output.send(UciOutput::Option(self.$field.to_uci()))?;
                    }
                )*
                Ok(())
            }

        }
    };
}

// Engine configuration parameters.
// Format: (field, "UCI Name", type, default, exposed_via_uci)
// Most tuning params use cfg!(feature = "tuning") so they're hidden in release builds.
define_config!(
    // --- Core UCI options (always exposed) ---
    (hash_size: i32, "Hash", UciOptionType::Spin { min: 1, max: 16384 }, 256, true),
    (move_overhead: i32, "Move Overhead", UciOptionType::Spin { min: 0, max: 5000 }, 20, true),
    (nnue: bool, "NNUE", UciOptionType::Check, true, true),

    (aspiration_window_size: i16, "Aspiration Window Size", UciOptionType::Spin { min: 10, max: 100 }, 40, cfg!(feature = "tuning")),
    (aspiration_window_widen: i16, "Aspiration Window Widening", UciOptionType::Spin { min: 2, max: 4 }, 2, cfg!(feature = "tuning")),
    (aspiration_window_depth: u8, "Aspiration Window Depth", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (aspiration_window_retries: i16, "Aspiration Window Retries", UciOptionType::Spin { min: 1, max: 5 }, 3, cfg!(feature = "tuning")),

    (history_max_value: i32, "History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 482, cfg!(feature = "tuning")),
    (history_reduction_threshold: i16, "History Reduction Threshold", UciOptionType::Spin { min: -512, max: 512 }, -12, cfg!(feature = "tuning")),
    (history_prune_threshold: i16, "History Prune Threshold", UciOptionType::Spin { min: -512, max: 512 }, -66, cfg!(feature = "tuning")),
    (history_min_move_index: i32, "History Min Move Index", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (history_bonus_multiplier: i32, "History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 13, cfg!(feature = "tuning")),
    (history_malus_multiplier: i32, "History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 7, cfg!(feature = "tuning")),

    (capture_history_max_value: i32, "Capture History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (capture_history_bonus_multiplier: i32, "Capture History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")),
    (capture_history_malus_multiplier: i32, "Capture History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 3, cfg!(feature = "tuning")),

    (continuation_max_value: i32, "Continuation Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (continuation_max_moves: usize, "Continuation Max Moves", UciOptionType::Spin { min: 1, max: 4 }, 4, cfg!(feature = "tuning")),
    (continuation_bonus_multiplier: i32, "Continuation Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 8, cfg!(feature = "tuning")),
    (continuation_malus_multiplier: i32, "Continuation Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 11, cfg!(feature = "tuning")),

    (quiet_check_bonus: i16, "Quiet Check Bonus", UciOptionType::Spin { min: 0, max: 2000 }, 980, cfg!(feature = "tuning")),

    (lmr_min_depth: u8, "LMR Min Depth", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),
    (lmr_divisor: i32, "LMR Divisor", UciOptionType::Spin { min: 100, max: 400 }, 230, cfg!(feature = "tuning")),
    (lmr_max_reduction_ratio: i32, "LMR Max Reduction Ratio", UciOptionType::Spin { min: 10, max: 100 }, 52, cfg!(feature = "tuning")),

    (nmp_min_depth: u8, "NMP Min Depth", UciOptionType::Spin { min: 2, max: 10 }, 4, cfg!(feature = "tuning")),
    (nmp_base_reduction: u8, "NMP Base Reduction", UciOptionType::Spin { min: 1, max: 10 }, 2, cfg!(feature = "tuning")),
    (nmp_depth_divisor: u8, "NMP Depth Divisor", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),
    (nmp_eval_margin: i16, "NMP Eval Margin", UciOptionType::Spin { min: 0, max: 500 }, 189, cfg!(feature = "tuning")),

    (lmp_max_depth: u8, "LMP Max Depth", UciOptionType::Spin { min: 0, max: 20 }, 8, cfg!(feature = "tuning")),
    (lmp_base_moves: i32, "LMP Base Moves", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),
    (lmp_depth_multiplier: i32, "LMP Depth Multiplier", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),
    (lmp_improving_reduction: i32, "LMP Improving Reduction", UciOptionType::Spin { min: 50, max: 100 }, 85, cfg!(feature = "tuning")),

    (futility_max_depth: u8, "Futility Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 4, cfg!(feature = "tuning")),
    (futility_base_margin: i16, "Futility Base Margin", UciOptionType::Spin { min: 10, max: 300 }, 145, cfg!(feature = "tuning")),
    (futility_depth_multiplier: i16, "Futility Depth Multiplier", UciOptionType::Spin { min: 10, max: 200 }, 97, cfg!(feature = "tuning")),

    (rfp_max_depth: u8, "RFP Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (rfp_base_margin: i16, "RFP Base Margin", UciOptionType::Spin { min: 10, max: 300 }, 116, cfg!(feature = "tuning")),
    (rfp_depth_multiplier: i16, "RFP Depth Multiplier", UciOptionType::Spin { min: 10, max: 200 }, 99, cfg!(feature = "tuning")),
    (rfp_improving_bonus: i16, "RFP Improving Bonus", UciOptionType::Spin { min: 10, max: 100 }, 50, cfg!(feature = "tuning")),

    (razor_max_depth: u8, "Razor Max Depth", UciOptionType::Spin { min: 1, max: 5 }, 3, cfg!(feature = "tuning")),
    (razor_base_margin: i16, "Razor Base Margin", UciOptionType::Spin { min: 100, max: 800 }, 512, cfg!(feature = "tuning")),
    (razor_depth_coefficient: i16, "Razor Depth Coefficient", UciOptionType::Spin { min: 100, max: 500 }, 293, cfg!(feature = "tuning")),

    (qs_delta_margin: i16, "QS Delta Margin", UciOptionType::Spin { min: 10, max: 500 }, 200, cfg!(feature = "tuning")),
    (qs_delta_material_threshold: i16, "QS Delta Material Threshold", UciOptionType::Spin { min: 100, max: 3000 }, 1500, cfg!(feature = "tuning")),

    (iid_reduction: u8, "IID Reduction", UciOptionType::Spin { min: 1, max: 10 }, 3, cfg!(feature = "tuning")),

    (iir_reduction: u8, "IIR Reduction", UciOptionType::Spin { min: 0, max: 4 }, 1, cfg!(feature = "tuning")),
    (iir_min_depth: u8, "IIR Min Depth", UciOptionType::Spin { min: 2, max: 10 }, 4, cfg!(feature = "tuning")),

    // SEE Pruning
    (see_prune_min_remaining_depth: u8, "SEE Prune Min Remaining Depth", UciOptionType::Spin { min: 0, max: 10 }, 1, cfg!(feature = "tuning")),
    (see_prune_max_depth: u8, "SEE Prune Max Depth", UciOptionType::Spin { min: 1, max: 10 }, 6, cfg!(feature = "tuning")),
    (see_prune_depth_margin: i16, "SEE Prune Depth Margin", UciOptionType::Spin { min: 10, max: 150 }, 77, cfg!(feature = "tuning")),
    (see_prune_min_attacker_value: i16, "SEE Prune Min Attacker Value", UciOptionType::Spin { min: 0, max: 500 }, 200, cfg!(feature = "tuning")),

    (piece_value_pawn_mg: f32, "Piece Value Pawn MG", UciOptionType::Spin { min: 50, max: 150 }, 98.0, cfg!(feature = "tuning")),
    (piece_value_pawn_eg: f32, "Piece Value Pawn EG", UciOptionType::Spin { min: 50, max: 150 }, 113.0, cfg!(feature = "tuning")),
    (piece_value_knight_mg: f32, "Piece Value Knight MG", UciOptionType::Spin { min: 250, max: 400 }, 325.0, cfg!(feature = "tuning")),
    (piece_value_knight_eg: f32, "Piece Value Knight EG", UciOptionType::Spin { min: 250, max: 400 }, 340.0, cfg!(feature = "tuning")),
    (piece_value_bishop_mg: f32, "Piece Value Bishop MG", UciOptionType::Spin { min: 250, max: 400 }, 335.0, cfg!(feature = "tuning")),
    (piece_value_bishop_eg: f32, "Piece Value Bishop EG", UciOptionType::Spin { min: 250, max: 400 }, 350.0, cfg!(feature = "tuning")),
    (piece_value_rook_mg: f32, "Piece Value Rook MG", UciOptionType::Spin { min: 400, max: 600 }, 510.0, cfg!(feature = "tuning")),
    (piece_value_rook_eg: f32, "Piece Value Rook EG", UciOptionType::Spin { min: 450, max: 650 }, 560.0, cfg!(feature = "tuning")),
    (piece_value_queen_mg: f32, "Piece Value Queen MG", UciOptionType::Spin { min: 800, max: 1200 }, 975.0, cfg!(feature = "tuning")),
    (piece_value_queen_eg: f32, "Piece Value Queen EG", UciOptionType::Spin { min: 800, max: 1300 }, 1020.0, cfg!(feature = "tuning")),

    (hce_tempo_bonus: i16, "HCE Tempo Bonus", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")),

    (hce_doubled_pawn_penalty: i16, "HCE Doubled Pawn Penalty", UciOptionType::Spin { min: 0, max: 100 }, 30, cfg!(feature = "tuning")),
    (hce_tripled_pawn_penalty: i16, "HCE Tripled Pawn Penalty", UciOptionType::Spin { min: 0, max: 150 }, 60, cfg!(feature = "tuning")),
    (hce_isolated_pawn_penalty: i16, "HCE Isolated Pawn Penalty", UciOptionType::Spin { min: 0, max: 100 }, 39, cfg!(feature = "tuning")),
    (hce_backward_pawn_penalty: i16, "HCE Backward Pawn Penalty", UciOptionType::Spin { min: 0, max: 100 }, 20, cfg!(feature = "tuning")),
    (hce_backward_pawn_half_open_penalty: i16, "HCE Backward Pawn Half Open Penalty", UciOptionType::Spin { min: 0, max: 50 }, 10, cfg!(feature = "tuning")),

    (hce_passed_pawn_linear: i16, "HCE Passed Pawn Linear", UciOptionType::Spin { min: 0, max: 20 }, 6, cfg!(feature = "tuning")),
    (hce_passed_pawn_quadratic: i16, "HCE Passed Pawn Quadratic", UciOptionType::Spin { min: 0, max: 10 }, 5, cfg!(feature = "tuning")),
    (hce_center_pawn_bonus: i16, "HCE Center Pawn Bonus", UciOptionType::Spin { min: 0, max: 50 }, 20, cfg!(feature = "tuning")),

    (hce_bishop_pair_bonus: i16, "HCE Bishop Pair Bonus", UciOptionType::Spin { min: 0, max: 150 }, 50, cfg!(feature = "tuning")),
    (hce_rook_open_file_bonus: i16, "HCE Rook Open File Bonus", UciOptionType::Spin { min: 0, max: 50 }, 15, cfg!(feature = "tuning")),
    (hce_rook_semi_open_file_bonus: i16, "HCE Rook Semi-Open File Bonus", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")),
    (hce_rook_seventh_rank_bonus: i16, "HCE Rook Seventh Rank Bonus", UciOptionType::Spin { min: 0, max: 50 }, 20, cfg!(feature = "tuning")),

    (hce_space_multiplier: i16, "HCE Space Multiplier", UciOptionType::Spin { min: 0, max: 10 }, 4, cfg!(feature = "tuning")),

    (hce_supported_minor_bonus: i16, "HCE Supported Minor Bonus", UciOptionType::Spin { min: 0, max: 20 }, 5, cfg!(feature = "tuning")),
    (hce_supported_major_bonus: i16, "HCE Supported Major Bonus", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")),

    (hce_king_shield_r1_bonus: i16, "HCE King Shield R1 Bonus", UciOptionType::Spin { min: 0, max: 50 }, 12, cfg!(feature = "tuning")),
    (hce_king_shield_r2_bonus: i16, "HCE King Shield R2 Bonus", UciOptionType::Spin { min: 0, max: 50 }, 6, cfg!(feature = "tuning")),

    (hce_king_open_file_penalty: i16, "HCE King Open File Penalty", UciOptionType::Spin { min: 0, max: 50 }, 24, cfg!(feature = "tuning")),
    (hce_king_semi_open_file_penalty: i16, "HCE King Semi Open File Penalty", UciOptionType::Spin { min: 0, max: 50 }, 12, cfg!(feature = "tuning")),
    (hce_king_thin_cover_penalty: i16, "HCE King Thin Cover Penalty", UciOptionType::Spin { min: 0, max: 50 }, 6, cfg!(feature = "tuning")),

    (hce_king_pressure_knight: i16, "HCE King Pressure Knight", UciOptionType::Spin { min: 0, max: 50 }, 12, cfg!(feature = "tuning")),
    (hce_king_pressure_bishop: i16, "HCE King Pressure Bishop", UciOptionType::Spin { min: 0, max: 50}, 14, cfg!(feature = "tuning")),
    (hce_king_pressure_rook: i16, "HCE King Pressure Rook", UciOptionType::Spin { min: 0, max: 50}, 18, cfg!(feature = "tuning")),
    (hce_king_pressure_queen: i16, "HCE King Pressure Queen", UciOptionType::Spin { min: 0, max: 50 }, 22, cfg!(feature = "tuning")),
    (hce_king_pressure_pawn: i16, "HCE King Pressure Pawn", UciOptionType::Spin { min: 0, max: 50 }, 8, cfg!(feature = "tuning")),

    (hce_king_central_penalty: i16, "HCE King Central Penalty", UciOptionType::Spin { min: 0, max: 50 }, 20, cfg!(feature = "tuning")),
    (hce_king_activity_bonus: i16, "HCE King Activity Bonus", UciOptionType::Spin { min: 0, max: 50 }, 14, cfg!(feature = "tuning")),

    (hce_threats_multiplier: i16, "HCE Threats Multiplier", UciOptionType::Spin { min: 0, max: 100 }, 50, cfg!(feature = "tuning")),

    (piece_repetition_base_penalty: i16, "Piece Repetition Base Penalty", UciOptionType::Spin { min: 0, max: 100 }, 10, cfg!(feature = "tuning")),
    (piece_repetition_min_phase: f32, "Piece Repetition Min Phase", UciOptionType::Spin { min: 0, max: 100 }, 50.0, cfg!(feature = "tuning")),
);

impl EngineConfig {
    pub fn get_piece_values(&self) -> PieceValues {
        PieceValues {
            pawn_value_mg: self.piece_value_pawn_mg.value,
            pawn_value_eg: self.piece_value_pawn_eg.value,
            knight_value_mg: self.piece_value_knight_mg.value,
            knight_value_eg: self.piece_value_knight_eg.value,
            bishop_value_mg: self.piece_value_bishop_mg.value,
            bishop_value_eg: self.piece_value_bishop_eg.value,
            rook_value_mg: self.piece_value_rook_mg.value,
            rook_value_eg: self.piece_value_rook_eg.value,
            queen_value_mg: self.piece_value_queen_mg.value,
            queen_value_eg: self.piece_value_queen_eg.value,
        }
    }

    pub fn get_hce_config(&self) -> HCEConfig {
        HCEConfig {
            tempo_bonus: self.hce_tempo_bonus.value,

            doubled_pawn_penalty: self.hce_doubled_pawn_penalty.value,
            tripled_pawn_penalty: self.hce_tripled_pawn_penalty.value,
            isolated_pawn_penalty: self.hce_isolated_pawn_penalty.value,
            backward_pawn_penalty: self.hce_backward_pawn_penalty.value,
            backward_pawn_half_open_penalty: self.hce_backward_pawn_half_open_penalty.value,
            passed_pawn_linear: self.hce_passed_pawn_linear.value,
            passed_pawn_quadratic: self.hce_passed_pawn_quadratic.value,
            center_pawn_bonus: self.hce_center_pawn_bonus.value,

            bishop_pair_bonus: self.hce_bishop_pair_bonus.value,
            rook_open_file_bonus: self.hce_rook_open_file_bonus.value,
            rook_semi_open_file_bonus: self.hce_rook_semi_open_file_bonus.value,
            rook_seventh_rank_bonus: self.hce_rook_seventh_rank_bonus.value,

            space_multiplier: self.hce_space_multiplier.value,

            supported_minor_bonus: self.hce_supported_minor_bonus.value,
            supported_major_bonus: self.hce_supported_major_bonus.value,

            king_shield_r1_bonus: self.hce_king_shield_r1_bonus.value,
            king_shield_r2_bonus: self.hce_king_shield_r2_bonus.value,
            king_open_file_penalty: self.hce_king_open_file_penalty.value,
            king_semi_open_file_penalty: self.hce_king_semi_open_file_penalty.value,
            king_thin_cover_penalty: self.hce_king_thin_cover_penalty.value,
            king_pressure_knight: self.hce_king_pressure_knight.value,
            king_pressure_bishop: self.hce_king_pressure_bishop.value,
            king_pressure_rook: self.hce_king_pressure_rook.value,
            king_pressure_queen: self.hce_king_pressure_queen.value,
            king_pressure_pawn: self.hce_king_pressure_pawn.value,
            king_central_penalty: self.hce_king_central_penalty.value,
            king_activity_bonus: self.hce_king_activity_bonus.value,

            threats_multiplier: self.hce_threats_multiplier.value,
        }
    }
}

/// A configuration parameter with optional UCI metadata.
/// If `uci` is Some, the parameter can be changed via UCI setoption.
#[derive(Debug, Clone)]
pub struct ConfigParam<T> {
    pub value: T,
    pub uci: Option<UciOption>,
}

impl<T> ConfigParam<T>
where
    T: FromStr + ToString + Clone,
    T::Err: std::fmt::Display,
{
    pub fn update_from_uci(&mut self, value: &str) -> Result<(), String> {
        if let Some(uci_meta) = &self.uci {
            uci_meta.option_type.validate(value)?;
        }

        let new_value = value
            .parse::<T>()
            .map_err(|e| format!("Parse error: {}", e))?;

        self.value = new_value;
        Ok(())
    }

    pub fn to_uci(&self) -> String {
        let uci_meta = self
            .uci
            .as_ref()
            .expect("UCI metadata required for UCI output");
        uci_meta.option_type.to_uci(uci_meta.name, &self.value)
    }
}
