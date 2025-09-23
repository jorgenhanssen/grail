use std::str::FromStr;
use uci::{UciOption, UciOptionType, UciOutput};

fn uci(include: bool, name: &'static str, option_type: UciOptionType) -> Option<UciOption> {
    if include {
        Some(UciOption { name, option_type })
    } else {
        None
    }
}

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
                match uci_name {
                    $($uci_name if $include => self.$field.update_from_uci(value),)*
                    _ => Err(format!("Unknown parameter: {}", uci_name)),
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

            pub fn get_piece_values(&self) -> evaluation::piece_values::PieceValues {
                evaluation::piece_values::PieceValues {
                    pawn_value_mg: self.hce_pawn_value_mg.value,
                    pawn_value_eg: self.hce_pawn_value_eg.value,
                    knight_value_mg: self.hce_knight_value_mg.value,
                    knight_value_eg: self.hce_knight_value_eg.value,
                    bishop_value_mg: self.hce_bishop_value_mg.value,
                    bishop_value_eg: self.hce_bishop_value_eg.value,
                    rook_value_mg: self.hce_rook_value_mg.value,
                    rook_value_eg: self.hce_rook_value_eg.value,
                    queen_value_mg: self.hce_queen_value_mg.value,
                    queen_value_eg: self.hce_queen_value_eg.value,
                }
            }
        }
    };
}

define_config!(
    // Standard UCI parameters
    (hash_size: i32, "Hash", UciOptionType::Spin { min: 1, max: 1024 }, 384, true),

    // Aspiration Windows - Search with tight bounds around expected score
    (aspiration_window_size: i16, "Aspiration Window Size", UciOptionType::Spin { min: 10, max: 100 }, 40, cfg!(feature = "tuning")), // Initial window size in centipawns
    (aspiration_window_widen: i16, "Aspiration Window Widening", UciOptionType::Spin { min: 1, max: 4 }, 2, cfg!(feature = "tuning")), // Factor to widen window on fail
    (aspiration_window_depth: u8, "Aspiration Window Depth", UciOptionType::Spin { min: 1, max: 10 }, 4, cfg!(feature = "tuning")), // Minimum depth to use aspiration
    (aspiration_window_retries: i16, "Aspiration Window Retries", UciOptionType::Spin { min: 1, max: 5 }, 2, cfg!(feature = "tuning")), // Max retries before full window

    // History Heuristic - Track move success/failure for ordering
    (history_max_value: i32, "History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")), // Maximum  history score (absolute value)
    (history_reduction_threshold: i16, "History Reduction Threshold", UciOptionType::Spin { min: -512, max: 512 }, -8, cfg!(feature = "tuning")), // Score below which to reduce moves
    (history_prune_threshold: i16, "History Prune Threshold", UciOptionType::Spin { min: -512, max: 512 }, -64, cfg!(feature = "tuning")), // Score below which to prune moves
    (history_min_move_index: i32, "History Min Move Index", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")), // Minimum move number for history pruning
    (history_bonus_multiplier: i32, "History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 13, cfg!(feature = "tuning")), // Scaling for successful moves
    (history_malus_multiplier: i32, "History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 2, cfg!(feature = "tuning")), // Scaling for failed moves

    // Capture History - Track capture move success for ordering
    (capture_history_max_value: i32, "Capture History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")), // Maximum capture history score (absolute value)
    (capture_history_bonus_multiplier: i32, "Capture History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 10, cfg!(feature = "tuning")), // Scaling for successful captures
    (capture_history_malus_multiplier: i32, "Capture History Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 2, cfg!(feature = "tuning")), // Scaling for failed captures

    // Continuation History - Track move sequences for ordering
    (continuation_max_value: i32, "Continuation Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")), // Maximum continuation score (absolute value)
    (continuation_max_moves: usize, "Continuation Max Moves", UciOptionType::Spin { min: 1, max: 4 }, 3, cfg!(feature = "tuning")), // Number of previous moves to consider
    (continuation_bonus_multiplier: i32, "Continuation Bonus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 9, cfg!(feature = "tuning")), // Scaling for successful continuations
    (continuation_malus_multiplier: i32, "Continuation Malus Multiplier", UciOptionType::Spin { min: 0, max: 30 }, 7, cfg!(feature = "tuning")), // Scaling for failed continuations

    // Late Move Reduction - Reduce search depth for later moves
    (lmr_min_depth: u8, "LMR Min Depth", UciOptionType::Spin { min: 1, max: 6 }, 3, cfg!(feature = "tuning")), // Minimum depth to apply LMR
    (lmr_divisor: i32, "LMR Divisor", UciOptionType::Spin { min: 100, max: 400 }, 230, cfg!(feature = "tuning")), // Formula divisor (2.30 scaled by 100)
    (lmr_max_reduction_ratio: i32, "LMR Max Reduction Ratio", UciOptionType::Spin { min: 10, max: 100 }, 50, cfg!(feature = "tuning")), // Max reduction as % of depth (half of depth as default)

    // Null Move Pruning - Skip a turn to test position strength
    (nmp_min_depth: u8, "NMP Min Depth", UciOptionType::Spin { min: 2, max: 6 }, 3, cfg!(feature = "tuning")), // Minimum depth to try null move
    (nmp_base_reduction: u8, "NMP Base Reduction", UciOptionType::Spin { min: 1, max: 4 }, 2, cfg!(feature = "tuning")), // Base depth reduction
    (nmp_depth_divisor: u8, "NMP Depth Divisor", UciOptionType::Spin { min: 2, max: 6 }, 3, cfg!(feature = "tuning")), // Divide depth by this for extra reduction
    (nmp_eval_margin: i16, "NMP Eval Margin", UciOptionType::Spin { min: 50, max: 500 }, 200, cfg!(feature = "tuning")), // Eval margin for reduction adjustment

    // Late Move Pruning - Prune quiet moves after a limit based on depth
    (lmp_max_depth: u8, "LMP Max Depth", UciOptionType::Spin { min: 4, max: 12 }, 8, cfg!(feature = "tuning")), // Maximum depth to apply LMP
    (lmp_base_moves: i32, "LMP Base Moves", UciOptionType::Spin { min: 1, max: 6 }, 2, cfg!(feature = "tuning")), // Base move limit for formula
    (lmp_depth_multiplier: i32, "LMP Depth Multiplier", UciOptionType::Spin { min: 1, max: 6 }, 2, cfg!(feature = "tuning")), // Depth scaling factor for triangular formula
    (lmp_improving_reduction: i32, "LMP Improving Reduction", UciOptionType::Spin { min: 70, max: 95 }, 85, cfg!(feature = "tuning")), // Limit percentage when not improving

    // Futility Pruning - Prune moves that can't improve alpha
    (futility_max_depth: u8, "Futility Max Depth", UciOptionType::Spin { min: 1, max: 6 }, 3, cfg!(feature = "tuning")), // Maximum depth to apply futility pruning
    (futility_base_margin: i16, "Futility Base Margin", UciOptionType::Spin { min: 50, max: 300 }, 150, cfg!(feature = "tuning")), // Base margin at depth 1
    (futility_depth_multiplier: i16, "Futility Depth Multiplier", UciOptionType::Spin { min: 50, max: 200 }, 100, cfg!(feature = "tuning")), // Additional margin per depth

    // Reverse Futility Pruning - Prune positions that are too good (static beta cutoff)
    (rfp_max_depth: u8, "RFP Max Depth", UciOptionType::Spin { min: 1, max: 6 }, 3, cfg!(feature = "tuning")), // Maximum depth to apply RFP
    (rfp_base_margin: i16, "RFP Base Margin", UciOptionType::Spin { min: 50, max: 300 }, 150, cfg!(feature = "tuning")), // Base margin at depth 1
    (rfp_depth_multiplier: i16, "RFP Depth Multiplier", UciOptionType::Spin { min: 50, max: 200 }, 100, cfg!(feature = "tuning")), // Additional margin per depth
    (rfp_improving_bonus: i16, "RFP Improving Bonus", UciOptionType::Spin { min: 20, max: 100 }, 50, cfg!(feature = "tuning")), // Margin reduction for improving positions

    // Razor Pruning - Reduce to quiescence search when position looks unpromising
    (razor_max_depth: u8, "Razor Max Depth", UciOptionType::Spin { min: 1, max: 6 }, 3, cfg!(feature = "tuning")), // Maximum depth to apply razor pruning
    (razor_base_margin: i16, "Razor Base Margin", UciOptionType::Spin { min: 200, max: 800 }, 512, cfg!(feature = "tuning")), // Base margin for razor formula
    (razor_depth_coefficient: i16, "Razor Depth Coefficient", UciOptionType::Spin { min: 100, max: 500 }, 293, cfg!(feature = "tuning")), // Coefficient for depth² scaling

    // Quiescence Search - Delta pruning and capture evaluation
    (qs_delta_margin: i16, "QS Delta Margin", UciOptionType::Spin { min: 50, max: 500 }, 200, cfg!(feature = "tuning")), // Safety margin for delta pruning
    (qs_delta_material_threshold: i16, "QS Delta Material Threshold", UciOptionType::Spin { min: 200, max: 3000 }, 1500, cfg!(feature = "tuning")), // Minimum material to enable delta pruning

    // Internal Iterative Deepening - Search with reduced depth when no TT move
    (iid_reduction: u8, "IID Reduction", UciOptionType::Spin { min: 1, max: 4 }, 2, cfg!(feature = "tuning")), // Depth reduction for IID search


    // HCE
    (hce_pawn_value_mg: f32, "HCE Pawn Value MG", UciOptionType::Spin { min: 50, max: 150 }, 98.0, cfg!(feature = "tuning")),
    (hce_pawn_value_eg: f32, "HCE Pawn Value EG", UciOptionType::Spin { min: 50, max: 150 }, 113.0, cfg!(feature = "tuning")),
    (hce_knight_value_mg: f32, "HCE Knight Value MG", UciOptionType::Spin { min: 250, max: 400 }, 325.0, cfg!(feature = "tuning")),
    (hce_knight_value_eg: f32, "HCE Knight Value EG", UciOptionType::Spin { min: 250, max: 400 }, 340.0, cfg!(feature = "tuning")),
    (hce_bishop_value_mg: f32, "HCE Bishop Value MG", UciOptionType::Spin { min: 250, max: 400 }, 335.0, cfg!(feature = "tuning")),
    (hce_bishop_value_eg: f32, "HCE Bishop Value EG", UciOptionType::Spin { min: 250, max: 400 }, 350.0, cfg!(feature = "tuning")),
    (hce_rook_value_mg: f32, "HCE Rook Value MG", UciOptionType::Spin { min: 400, max: 600 }, 510.0, cfg!(feature = "tuning")),
    (hce_rook_value_eg: f32, "HCE Rook Value EG", UciOptionType::Spin { min: 450, max: 650 }, 560.0, cfg!(feature = "tuning")),
    (hce_queen_value_mg: f32, "HCE Queen Value MG", UciOptionType::Spin { min: 800, max: 1200 }, 975.0, cfg!(feature = "tuning")),
    (hce_queen_value_eg: f32, "HCE Queen Value EG", UciOptionType::Spin { min: 800, max: 1300 }, 1020.0, cfg!(feature = "tuning")),

);

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
