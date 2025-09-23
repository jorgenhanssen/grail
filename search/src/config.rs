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
        }
    };
}

define_config!(
    // Standard UCI parameters
    (hash_size: i32, "Hash", UciOptionType::Spin { min: 1, max: 1024 }, 384, true),

    // Tuning parameters
    (aspiration_window_size: i16, "Aspiration Window Size", UciOptionType::Spin { min: 10, max: 100 }, 40, cfg!(feature = "tuning")),
    (aspiration_window_widen: i16, "Aspiration Window Widening", UciOptionType::Spin { min: 1, max: 4 }, 2, cfg!(feature = "tuning")),
    (aspiration_window_depth: u8, "Aspiration Window Depth", UciOptionType::Spin { min: 1, max: 10 }, 4, cfg!(feature = "tuning")),
    (aspiration_window_retries: i16, "Aspiration Window Retries", UciOptionType::Spin { min: 1, max: 5 }, 2, cfg!(feature = "tuning")),

    (history_max_value: i32, "History Max Value", UciOptionType::Spin { min: 128, max: 1024 }, 512, cfg!(feature = "tuning")),
    (history_reduction_threshold: i16, "History Reduction Threshold", UciOptionType::Spin { min: -200, max: 200 }, -8, cfg!(feature = "tuning")),
    (history_prune_threshold: i16, "History Prune Threshold", UciOptionType::Spin { min: -200, max: 0 }, -64, cfg!(feature = "tuning")),
    (history_min_move_index: i32, "History Min Move Index", UciOptionType::Spin { min: 1, max: 10 }, 5, cfg!(feature = "tuning")),
    (history_bonus_multiplier: i32, "History Bonus Multiplier", UciOptionType::Spin { min: 0, max: 50 }, 13, cfg!(feature = "tuning")),
    (history_malus_multiplier: i32, "History Malus Multiplier", UciOptionType::Spin { min: 0, max: 50 }, 2, cfg!(feature = "tuning")),

    // TODO: Add other parameters

    // (threads: i32, "Threads", UciOptionType::Spin { min: 1, max: 192 }, 1, true),
    // (ponder: bool, "Ponder", UciOptionType::Check, false, true),

    // // Search parameters - only tunable with feature flag
    // (aspiration_window: i32, "Aspiration Window", UciOptionType::Spin { min: 10, max: 200 }, 50, cfg!(feature = "tuning")),
    // (null_move_reduction: i32, "Null Move Reduction", UciOptionType::Spin { min: 1, max: 6 }, 3, cfg!(feature = "tuning")),
    // (late_move_pruning: i32, "Late Move Pruning", UciOptionType::Spin { min: 50, max: 1000 }, 300, cfg!(feature = "tuning")),
    // (futility_pruning: i32, "Futility Pruning", UciOptionType::Spin { min: 50, max: 500 }, 200, cfg!(feature = "tuning")),

    // // Evaluation parameters - only tunable with feature flag
    // (piece_square_bonus: i32, "Piece Square Bonus", UciOptionType::Spin { min: 0, max: 300 }, 100, cfg!(feature = "tuning")),
    // (mobility_bonus: i32, "Mobility Bonus", UciOptionType::Spin { min: 0, max: 200 }, 50, cfg!(feature = "tuning")),
    // (king_safety: i32, "King Safety", UciOptionType::Spin { min: 0, max: 200 }, 75, cfg!(feature = "tuning")),
    // (king_mg_weight: i32, "King MG Weight", UciOptionType::Spin { min: 0, max: 100 }, 50, cfg!(feature = "tuning")),

    // // Debug parameters
    // (debug_search: bool, "Debug Search", UciOptionType::Check, false, true),
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
